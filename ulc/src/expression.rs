use std::fmt;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Variable(i32), // de Bruijn index
    Compound(Box<Expression>, Box<Expression>),
    Lambda(Box<Expression>, String),
}

use Expression::*;

// Dropping a Box-linked tree recursively overflows the stack on deep terms (WebAssembly's
// call stack is small), so tear the tree down with an explicit stack instead.
impl Drop for Expression {
    fn drop(&mut self) {
        let mut stack = Vec::new();
        detach_children(self, &mut stack);

        while let Some(mut node) = stack.pop() {
            detach_children(&mut node, &mut stack);
            // `node` now has only leaf children, so dropping it here recurses at most one level.
        }
    }
}

// Moves the non-leaf children of `expr` onto `stack`, leaving leaf placeholders behind.
fn detach_children(expr: &mut Expression, stack: &mut Vec<Box<Expression>>) {
    fn take(child: &mut Box<Expression>, stack: &mut Vec<Box<Expression>>) {
        if !matches!(**child, Variable(_)) {
            stack.push(std::mem::replace(child, Box::new(Variable(0))));
        }
    }

    match expr {
        Variable(_) => {}
        Compound(left, right) => {
            take(left, stack);
            take(right, stack);
        }
        Lambda(body, _) => take(body, stack),
    }
}

impl Expression {
    // Height of the expression tree (a lone variable has depth 0). Iterative, so it is safe
    // to call on terms too deep for the recursive functions below.
    pub fn depth(&self) -> usize {
        let mut max = 0;
        let mut stack = vec![(self, 0)];

        while let Some((expr, d)) = stack.pop() {
            max = max.max(d);
            match expr {
                Variable(_) => {}
                Compound(left, right) => {
                    stack.push((left, d + 1));
                    stack.push((right, d + 1));
                }
                Lambda(body, _) => stack.push((body, d + 1)),
            }
        }

        max
    }

    // Shifts the variables in an Expression which are greater than or equal to the offset.
    // i - the amount to be added to the variables.
    // offset - determines which variables are shifted.
    // returns: an Expression with all appropriate variables shifted.
    fn shift (&self, i: i32, offset: i32) -> Self {
        match self {
            Variable ( value ) => Variable (
                    if *value < offset {
                        *value
                    } else {
                        *value + i
                    }
                ),

            Compound ( left, right ) => Compound ( 
                Box::new(left.shift(i, offset)),
                Box::new(right.shift(i, offset))
            ),

            Lambda ( body, name) => Lambda(
                Box::new(body.shift(i, offset + 1)),
                name.clone()
            ),
        }
    }

    // Substitutes an Expression in place of all variables with a given value.
    // expr - the Expression to be substituted in.
    // i - the value of variables for which expr should be substituted.
    // Returns: An Expression with all variables whose value matches substituted for expr.
    fn substitute (&self, expr: &Expression, i: i32) -> Self {
        match self {
            Variable ( value ) => if *value == i {
                    expr.clone()
                } else {
                    self.clone()
                },

            Compound ( left, right ) => Compound (
                Box::new(left.substitute(expr, i)), 
                Box::new(right.substitute(expr, i)),
            ),

            Lambda ( body, name )=> Lambda (
                Box::new(body.substitute(&expr.shift(1, 0), i + 1)),
                name.clone()
            ),
        }
    }

    // Implements a single step of normal order reduction on an Expression.
    // If the Expression is reducable, returns the reduct. Else returns None
    // This has the following rules:
    // If expr is a variable, it's irreducible.
    // If expr has the form λ.M, then try to perform reduction on M.
    // If expr has the form (λ.M)N, then beta reduce it.
    // If expr has the form MN were M is not a lambda, then try to reduce M first, then try N if M is irreducible.
    // Returns: The new expression after one step of reduction
    // Else returns None.
    pub fn normal_order_reduce(&self) -> Option<Self> {
        match self {
            Variable (..) => None,

            Lambda ( body, name) => match body.normal_order_reduce() {
                Some ( value ) => Some(Lambda(Box::new(value), name.clone())),
                _ => None
            },

            Compound ( left, right ) => match left.as_ref() {
                Lambda ( body, _ ) => Some(
                    body.substitute(&right.shift(1, 0), 0).shift(-1, 0)
                ),

                left_unboxed => match left_unboxed.normal_order_reduce() {
                    Some ( value ) => Some(Compound ( Box::new(value), right.clone() )),

                    _ => match right.normal_order_reduce() {
                        Some ( value ) => Some(Compound ( left.clone(), Box::new(value) )),
                        _ => None
                    }
                }
            },
        }
    }
}

// Display using de Bruijn indices
impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Variable (value) => write!(f, "{}", value),

            Compound ( left, right ) => write!(f, "({} {})", left, right),

            Lambda ( body, _ ) => write!(f, "λ.{}", body),

        }
    }
}

// Displays an Expression using variable names instead of de Bruijn indices
#[derive(Debug, PartialEq)]
pub struct PrettifiedExpression {
    expr: Expression,
    // running_expr: &Expression, // stores current expression when going down recursive tree for display
    free_vars: Vec<String>, // negative of absolute binding level -> name
}

struct DisplayEnv {
    bound_vars: Vec<String>, // absolute binding level -> name 
    name_binding_levels: HashMap<String, HashMap<i32, usize>>, // name -> absolute binding level -> name-specific binding level
    current_name_binding_levels: HashMap<String, usize>, // name -> name-specific binding level
}

impl DisplayEnv {
    fn new() -> Self {
        Self {
            bound_vars: Vec::new(),
            name_binding_levels: HashMap::new(),
            current_name_binding_levels: HashMap::new(),
        }
    }
}

impl PrettifiedExpression {
    pub fn new(expr: Expression, free_vars: Vec<String>) -> Self {
        Self {
            expr,
            free_vars,
        }
    }
    
    pub fn get_expr(&self) -> &Expression {
        &self.expr
    }
    
    pub fn set_expr(&mut self, expr: Expression) {
        self.expr = expr;
    }

    pub fn free_vars(&self) -> &[String] {
        &self.free_vars
    }

    fn fmt_pretty(&self, expr: &Expression, env: &mut DisplayEnv) -> String {
        self.init_free_var_binding_levels(env);

        match expr {
            Variable( de_bruijn_index ) => self.fmt_var(*de_bruijn_index, env),
            Lambda ( body, name ) => self.fmt_lambda(body, name, env),
            Compound ( left, right ) => self.fmt_compound(left, right, env),
        }   
    }

    fn init_free_var_binding_levels(&self, env: &mut DisplayEnv) {
        // start: initialize display env based on free vars
        if env.name_binding_levels.is_empty() {
            for (i, name) in self.free_vars.iter().enumerate() {
                let mut map = HashMap::new();
                map.insert(-(i as i32), 0);
                env.name_binding_levels.insert(name.clone(), map);
            }
        }
    }

    fn fmt_var(&self, index: i32, env: &DisplayEnv) -> String {
        let mut result = String::new();

        // Step 1: compute absolute binding level of variable
        // # of bound vars = current abs binding level
        // subtract de Bruijn index
        let current_absolute_binding_level = env.bound_vars.len() as i32;
        let var_absolute_binding_level = current_absolute_binding_level - index;

        // how should nameless free var be handled?
        let default = "<FV>".to_string();

        // Step 2: get name
        let name = if var_absolute_binding_level > 0 {
            // Case bound var
            env.bound_vars.get(index as usize).unwrap()
        } else {
            // Case free var (abs binding level is 0 or negative)
            self.free_vars.get((-var_absolute_binding_level) as usize).unwrap_or(&default)
        };

        result.push_str(name);

        let default_abs_to_name = HashMap::new();

        // Step 3: current name binding level
        let abs_to_name = env.name_binding_levels.get(name).unwrap_or(&default_abs_to_name);

        let current_name_binding_level = env.current_name_binding_levels.get(name)
            .unwrap_or(&0);
    
        // Step 4: get name binding level of var
        let var_name_binding_level = abs_to_name.get(&var_absolute_binding_level).unwrap_or(&0);

        // Step 5: compute scope index
        let scope_index = current_name_binding_level.checked_sub(*var_name_binding_level).unwrap();
        
        if scope_index > 0 {
            result.push_str(&format!("^{}", scope_index));
        }

        result
    }

    fn fmt_lambda(&self, body: &Expression, name: &String, env: &mut DisplayEnv) -> String {
        let mut result = String::new();
        result.push_str(&format!("λ{}", name));

        let mut original_name_binding_levels = HashMap::new();
        lambda_env_track(name, &mut original_name_binding_levels, env);
        let mut new_expr = body;
        let start_absolute_binding_level = env.bound_vars.len();

        loop {
            if let Lambda ( body, name ) = new_expr {
                result.push_str(&format!(" {}", name));
                lambda_env_track(name, &mut original_name_binding_levels, env);
                new_expr = body;
            } else {
                let fmt_str = if let Variable(_) = new_expr {
                    format!(".{}", self.fmt_pretty(new_expr, env))
                } else {
                    format!(".({})", self.fmt_pretty(new_expr, env))
                };

                break result.push_str(&fmt_str);
            }
        }

        clean_env(original_name_binding_levels, start_absolute_binding_level, env);

        result
    }

    fn fmt_compound(&self, left: &Expression, right: &Expression, env: &mut DisplayEnv) -> String {
        let mut result = String::new();

        result.push_str(&format!("{} ", self.fmt_pretty(left, env)));

        if let Compound(_, _) = right {
            // If right is a compound put parentheses around it
            result.push_str(&format!("({})", self.fmt_pretty(right, env)));
        } else {
            result.push_str(&format!("{}", self.fmt_pretty(right, env)));
        }

        result
    }
}

fn lambda_env_track(name: &String, original_name_binding_levels: &mut HashMap<String, usize>, env: &mut DisplayEnv) {
    // TODO: is inserting to a vector this way efficient? (N0)
    env.bound_vars.insert(0, name.into());
    // We need: current absolute binding level + current name binding level.
    let name_abs_rel =   env.name_binding_levels.entry(name.clone()).or_insert(HashMap::new());

    // absolute binding level = size of env
    let absolute_binding_level = env.bound_vars.len();
    let current_name_binding_level = env.current_name_binding_levels.entry(name.clone()).or_insert(0);

    original_name_binding_levels.insert(name.clone(), *current_name_binding_level);

    *current_name_binding_level += 1;

    name_abs_rel.insert(absolute_binding_level as i32, *current_name_binding_level);
}

fn clean_env(original_name_binding_levels: HashMap<String, usize>, start_absolute_binding_level: usize, env: &mut DisplayEnv) {
    // gets rid of the variables added during the lambda scope
    let absolute_binding_level = env.bound_vars.len();
    let abs_binding_level_range = absolute_binding_level - start_absolute_binding_level + 1;

    env.bound_vars.drain(0..abs_binding_level_range);

    for (name, orig_name_binding_level) in original_name_binding_levels {
        // restore the original name binding level 
        if let Some(level) = env.current_name_binding_levels.get_mut(&name) {
            *level = orig_name_binding_level;
        }
    }
}

impl fmt::Display for PrettifiedExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut env = DisplayEnv::new();

        write!(f, "{}", self.fmt_pretty(&self.expr, &mut env))
    }
}

#[cfg(test)]
// TODO: Try to reduce the duplication.
mod tests {
    use super::{Expression::{self, *}, PrettifiedExpression};

    // Creates a lambda expression with a default (empty) variable name
    fn create_lambda(body: Expression) -> Expression {
        Lambda(Box::new(body), "".into())
    }

    // Variable shift tests
    #[test]
    fn shift_var_below_offset() {
        let var1 = Variable(1);
        let var2 = Variable(2);
        let var3 = Variable(-1);

        assert_eq!(var1.shift(3, 2), Variable(1));
        assert_eq!(var2.shift(1, 4), Variable(2));
        assert_eq!(var3.shift(-10, 0), Variable(-1));
    }

    #[test]
    fn shift_var_eq_offset() {
        let var1 = Variable(1);
        let var2 = Variable(2);
        let var3 = Variable(-1);

        assert_eq!(var1.shift(3, 1), Variable(4));
        assert_eq!(var2.shift(1, 2), Variable(3));
        assert_eq!(var3.shift(-10, -1), Variable(-11));
    }

    #[test]
    fn shift_var_above_offset() {
        let var1 = Variable(1);
        let var2 = Variable(2);
        let var3 = Variable(-1);

        assert_eq!(var1.shift(3, 0), Variable(4));
        assert_eq!(var2.shift(1, 0), Variable(3));
        assert_eq!(var3.shift(-10, -10), Variable(-11));
    }

    // Lambda shift tests
    #[test]
    fn shift_lambda_below_offset() {
        let la1 = create_lambda(Variable(0));
        let la2 = create_lambda(Variable(-1));
        let la3 = create_lambda(la1.clone());

        assert_eq!(la1.shift(1, 0), la1);
        assert_eq!(la2.shift(1, -1), la2);
        assert_eq!(la3.shift(1, -1), la3);
    }

    #[test]
    fn shift_create_lambda_eq_offset() {
        let la1 = create_lambda(Variable(0));
        let la2 = create_lambda(Variable(-1));
        let la3 = create_lambda(la1.clone());

        assert_eq!(la1.shift(1, -1), create_lambda(Variable(1)));
        assert_eq!(la2.shift(1, -2), create_lambda(Variable(0)));
        assert_eq!(la3.shift(1, -2), create_lambda(create_lambda(Variable(1))));
    }

    #[test]
    fn shift_lambda_above_offset() {
        let la1 = create_lambda(Variable(0));
        let la2 = create_lambda(Variable(-1));
        let la3 = create_lambda(la1.clone());

        assert_eq!(la1.shift(1, -2), create_lambda(Variable(1)));
        assert_eq!(la2.shift(1, -3), create_lambda(Variable(0)));
        assert_eq!(la3.shift(1, -3), create_lambda(create_lambda(Variable(1))));
    }

    // Compound shift tests
    #[test]
    fn shift_compound() {
        let comp = Compound(Box::new(Variable(0)), Box::new(Variable(3)));
        
        assert_eq!(comp.shift(1, 1), Compound(Box::new(Variable(0)), Box::new(Variable(4))));
        assert_eq!(comp.shift(1, 0), Compound(Box::new(Variable(1)), Box::new(Variable(4))));
    }

    // Variable substitute tests
    #[test]
    fn sub_variable_neq() {
        let var = Variable(0);
        let expr = Compound(
            Box::new(create_lambda(Variable(1))),
            Box::new(Variable(3)),
        );
        
        assert_eq!(var.substitute(&expr, 2), var);
        assert_eq!(var.substitute(&expr, -1), var);
    }

    #[test]
    fn sub_variable_eq() {
        let var = Variable(0);
        let expr = Compound(
            Box::new(create_lambda(Variable(1))),
            Box::new(Variable(3)),
        );

        assert_eq!(var.substitute(&expr, 0), expr);
    }

    // Lambda substitute tests
    #[test]
    fn sub_lambda_neq() {
        let la = create_lambda(Variable(0));
        let expr = Compound(
            Box::new(create_lambda(Variable(1))),
            Box::new(Variable(3)),
        );

        assert_eq!(la.substitute(&expr, 0), la);
    }

    #[test]
    fn sub_lambda_eq() {
        let la = create_lambda(Variable(1));
        let expr = Compound(
            Box::new(create_lambda(Variable(1))),
            Box::new(Variable(3)),
        );

        assert_eq!(la.substitute(&expr, 0), create_lambda(expr.shift(1, 0)));
    }

    // Compound substitute tests
    #[test]
    fn sub_compound_different_vars() {
        let var = Variable(0);
        let var1 = Variable(1);
        let expr = Compound(
            Box::new(create_lambda(Variable(1))),
            Box::new(Variable(3)),
        );

        let comp = Compound(Box::new(var), Box::new(var1));
        assert_eq!(comp.substitute(&expr, 1), Compound(Box::new(Variable(0)), Box::new(expr)));
    }

    #[test]
    fn sub_compound_same_var() {
        let expr = Compound(
            Box::new(create_lambda(Variable(1))),
            Box::new(Variable(3)),
        );

        let comp = Compound(Box::new(Variable(0)), Box::new(Variable(0)));

        assert_eq!(comp.substitute(&expr, 0), Compound(Box::new(expr.clone()), Box::new(expr)));
    }

    // Evaluation tests
    #[test]
    fn evaluate_beta_reduction() {
        let expr = Compound(
            Box::new(create_lambda(Variable(1))),
            Box::new(Variable(3)),
        );

        let comp = Compound(Box::new(create_lambda(Variable(0))), Box::new(expr.clone()));

        assert_eq!(comp.normal_order_reduce().unwrap(), expr)
    }

    // Left evaluation should occur before right evaluation
    #[test]
    fn evaluate_left_redex() {
        let left = Compound(
            Box::new(create_lambda(Variable(0))),
            Box::new(Variable(1))
        );

        let right = Compound(
            Box::new(create_lambda(Variable(0))),
            Box::new(Variable(2))
        );

        let comp = Compound(
            Box::new(left),
            Box::new(right.clone())
        );

        let expected = Compound(
            Box::new(Variable(1)),
            Box::new(right.clone())
        );

        assert_eq!(comp.normal_order_reduce().unwrap(), expected);
    }

    // If there is a redex only in the right hand side, it should be evaluated first.
    #[test]
    fn evaluate_right_redex() {

        let right = Compound(
            Box::new(create_lambda(Variable(0))),
            Box::new(Variable(2))
        );

        let comp = Compound(
            Box::new(Variable(0)),
            Box::new(right.clone())
        );

        let expected = Compound(
            Box::new(Variable(0)),
            Box::new(Variable(2))
        );

        assert_eq!(comp.normal_order_reduce().unwrap(), expected);
    }

    // A redex inside a lambda should be evaluated
    #[test]
    fn evaluate_lambda_redex() {
        let body = Compound(Box::new(create_lambda(Variable(0))), Box::new(Variable(1)));

        let lambda = create_lambda(body);

        let expected = create_lambda(Variable(1));

        assert_eq!(lambda.normal_order_reduce().unwrap(), expected);
    }

    // If there is no redex it ought to be unchanged
    #[test]
    fn evaluate_compound_no_redex() {
        let expr = Compound(
            Box::new(Variable(0)),
            Box::new(create_lambda(Variable(0))),
        );

        assert_eq!(expr.normal_order_reduce(), None);
    }

    // For a variable there is also no redex and it should be unchanged
    #[test]
    fn evaluate_variable_no_redex() {
        let var = Variable(0);

        assert_eq!(var.normal_order_reduce(), None);
    }

    // Test Expression display
    #[test]
    fn display_var() {
        let var = Variable(42);

        assert_eq!(format!("{}", var), "42");
    }

    #[test]
    fn display_compound() {
        let expr = Compound(
            Box::new(Variable(0)),
            Box::new(Variable(3))
        );

        assert_eq!(format!("{}", expr), "(0 3)");
    }

    #[test]
    fn display_lambda() {
        let expr = Lambda(Box::new(Variable(0)), String::new());

        assert_eq!(format!("{}", expr), "λ.0");
    }

    #[test]
    fn display_addition() {
        // ((λm.λn.λf.λx.((m f) ((n f) x))
        let expr = Lambda(Box::new(
            Lambda(Box::new(
                Lambda(Box::new(
                    Lambda(Box::new(
                        Compound(
                            Box::new(
                                Compound(
                                    Box::new(Variable(3)),
                                    Box::new(Variable(1))
                                )),

                                Box::new(
                                    Compound(
                                        Box::new(Compound(Box::new(Variable(2)), Box::new(Variable(1)))),
                                        Box::new(Variable(0))
                                    )
                                )
                            )
                        ), "x".to_string())
                ), "f".to_string())
            ), "n".to_string())
        ), "m".to_string());

        assert_eq!(format!("{}", expr), "λ.λ.λ.λ.((3 1) ((2 1) 0))");        
    }

    // Test pretty display with syntactic sugar
    fn test_pretty(expr: Expression, free_var_names: Vec<String>, expected: &str) {
        let pretty = PrettifiedExpression::new(expr, free_var_names);
        assert_eq!(format!("{:#}", pretty), expected);
    }

    #[test]
    fn pretty_display_free_var() {
        test_pretty(Variable(0), vec!["theAnswer".into()], "theAnswer");
    }

    #[test]
    fn pretty_display_free_var_out_of_scope_index() {
        test_pretty(Variable(42), vec!["theAnswer".into()], "<FV>");
    }

    #[test]
    fn pretty_display_compound_left_associative() {
        test_pretty(
            Compound(
                Box::new(Variable(1)),
                Box::new(Variable(0))
            ), 
            vec!["theQuestion".into(), "theAnswer".into()], 
            "theAnswer theQuestion"
        );

        test_pretty(
            Compound(
                Box::new(Compound(
                    Box::new(Variable(1)),
                    Box::new(Variable(0))
                )),
                Box::new(Variable(2))
            ),
            vec!["theQuestion".into(), "theAnswer".into(), "thePuzzle".into()],
            "theAnswer theQuestion thePuzzle"
        );

        test_pretty(
            Compound(
                Box::new(Compound(
                    Box::new(Compound(
                        Box::new(Variable(3)),
                        Box::new(Variable(1))
                    )),
                    Box::new(Variable(2))
                )),
                Box::new(Variable(1))
            ),
            vec!["theQuestion".into(), "theAnswer".into(), "thePuzzle".into(), "theSphynx".into()],
            "theSphynx theAnswer thePuzzle theAnswer"
        );
    }

    #[test]
    fn pretty_display_compound_non_left_associative() {
        test_pretty(
            Compound(
                Box::new(Compound(
                    Box::new(Variable(1)),
                    Box::new(Compound(
                        Box::new(Variable(1)),
                        Box::new(Variable(2))
                    ))
                )),
                Box::new(Variable(0))
            ),
            vec!["🤖".into(), "👻".into(), "🧠".into()],
            "👻 (👻 🧠) 🤖"
        );
    }

    #[test]
    fn pretty_display_lambda() {
        test_pretty(
            Lambda(Box::new(Variable(0)), "Null".to_string()),
            vec!["Null".into()],
            "λNull.Null"
        );
        
        test_pretty(
            Lambda(
                Box::new(Lambda(
                    Box::new(Variable(0)),
                    "x".to_string()
                )),
                "Null".to_string()
            ),
            Vec::new(),
            "λNull x.x"
        );

        test_pretty(
            Lambda(
                Box::new(Lambda(
                    Box::new(Lambda(
                        Box::new(Variable(1)),
                        "zed".to_string()
                    )),
                    "x".to_string()
                )),
                "Null".to_string()
            ),
            Vec::new(),
            "λNull x zed.x"
        );
    }

    #[test]
    fn pretty_display_addition() {
        test_pretty(
            Lambda(Box::new(
                Lambda(Box::new(
                    Lambda(Box::new(
                        Lambda(Box::new(
                            Compound(
                                Box::new(
                                    Compound(
                                        Box::new(Variable(3)),
                                        Box::new(Variable(1))
                                    )),
    
                                    Box::new(
                                        Compound(
                                            Box::new(Compound(Box::new(Variable(2)), Box::new(Variable(1)))),
                                            Box::new(Variable(0))
                                        )
                                    )
                                )
                            ), "x".to_string())
                    ), "f".to_string())
                ), "n".to_string())
            ), "m".to_string()),
            Vec::new(),
            "λm n f x.(m f (n f x))"
        );      
    }

    // Makes sure that the outer environment and its variables are accessible from within a lambda expression
    // Previous bug made this not the case.
    #[test]
    fn pretty_nested_lambda_outer_env() {
        // test with free vars
        test_pretty(
            Compound(
                Box::new(Variable(0)),
                Box::new(Lambda(
                    Box::new(Variable(1)),
                    "x".to_string()
                ))
            ),
            vec!["Eureka!".into()],
            "Eureka! λx.Eureka!"
        );

        // test with nested lambdas
        test_pretty(
            Lambda(
                Box::new(Compound(
                    Box::new(Lambda(
                        Box::new(Variable(0)),
                        "y".to_string()
                    )),
                    Box::new(Lambda(
                        Box::new(Variable(1)),
                        "y".to_string()
                    ))
                )
                ),
                "x".to_string()
            ),
            vec![],
            "λx.(λy.y λy.x)"
        );
        // λf x.(λx.(f (f (f x))) (λf x.(f (f x)) f x))
    }

    // test with scope indices
    #[test]
    fn pretty_display_scope_indices() {
        test_pretty(
            Expression::Lambda(
                Box::new(
                    Expression::Lambda(
                        Box::new(Expression::Variable(1)),
                    "x".to_string())),
                "x".to_string(),
            ),
            vec![],
            "λx x.x^1",
        );

        test_pretty(
            Expression::Lambda(
                Box::new(
                    Expression::Lambda(Box::new(
                        Expression::Lambda(Box::new(
                            Expression::Lambda(Box::new(
                                Compound(
                                    Box::new(
                                        Expression::Lambda(Box::new(
                                            Expression::Variable(4),
                                        ), "x".to_string()),
                                    ),
                                    Box::new(Expression::Lambda(Box::new(
                                        Compound(
                                            Box::new(Compound(
                                                Box::new(Expression::Variable(0)),
                                                Box::new(Expression::Variable(3)),
                                            )),
                                            Box::new(Expression::Variable(5))
                                        )),
                                    "y".to_string()))
                                )), "z".to_string())
                        ), "x".to_string())
                    ), "y".to_string())
                ), "x".to_string()),
            vec!["y".to_string()],
            "λx y x z.(λx.x^2 λy.(y y^1 y^2))",
        );
    }
}
