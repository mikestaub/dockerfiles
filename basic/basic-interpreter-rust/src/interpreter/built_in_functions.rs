use crate::common::*;
use crate::interpreter::context_owner::ContextOwner;
use crate::interpreter::{Interpreter, InterpreterError, Result, Stdlib};
use crate::linter::{QualifiedName, TypeQualifier};
use crate::variant::Variant;

impl<S: Stdlib> Interpreter<S> {
    pub fn run_built_in_function(
        &mut self,
        function_name: &QualifiedName,
        pos: Location,
    ) -> Result<()> {
        if function_name == &QualifiedName::new("ENVIRON", TypeQualifier::DollarString) {
            let v = self.context_mut().demand_sub().pop_front_unnamed(pos)?;
            match v {
                Variant::VString(env_var_name) => {
                    let result = self.stdlib.get_env_var(&env_var_name);
                    self.context_mut()
                        .demand_sub()
                        .set_function_result(Variant::VString(result));
                    Ok(())
                }
                _ => Err(InterpreterError::new_with_pos(
                    "Type mismatch at ENVIRON$",
                    pos,
                )),
            }
        } else if function_name == &QualifiedName::new("_Undefined_", TypeQualifier::PercentInteger)
        {
            loop {
                match self.context_mut().demand_sub().try_pop_front_unnamed(pos)? {
                    Some(v) => match v {
                        Variant::VString(_) => {
                            return Err(InterpreterError::new_with_pos("Type mismatch", pos));
                        }
                        _ => (),
                    },
                    None => {
                        break;
                    }
                }
            }
            self.context_mut()
                .demand_sub()
                .set_function_result(Variant::VInteger(0));
            Ok(())
        } else {
            panic!("Unknown function {:?}", function_name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::*;
    use crate::assert_has_variable;
    use crate::interpreter::Stdlib;
    use crate::variant::Variant;

    #[test]
    fn test_function_call_environ() {
        let program = r#"
        X$ = ENVIRON$("abc")
        Y$ = ENVIRON$("def")
        "#;
        let mut stdlib = MockStdlib::new();
        stdlib.set_env_var("abc".to_string(), "foo".to_string());
        let interpreter = interpret_with_stdlib(program, stdlib);
        assert_has_variable!(interpreter, "X$", "foo");
        assert_has_variable!(interpreter, "Y$", "");
    }
}
