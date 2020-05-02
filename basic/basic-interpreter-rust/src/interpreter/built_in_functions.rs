use crate::common::*;
use crate::interpreter::context_owner::ContextOwner;
use crate::interpreter::{
    Instruction, InstructionGenerator, Interpreter, InterpreterError, Result, Stdlib, Variant,
};
use crate::parser::{ExpressionNode, Name, NameNode};

pub fn is_built_in_function(function_name: &Name) -> bool {
    function_name == &Name::from("ENVIRON$")
}

impl<S: Stdlib> Interpreter<S> {
    fn _do_environ_function(
        &mut self,
        function_name: &NameNode,
        args: &Vec<ExpressionNode>,
    ) -> Result<Variant> {
        if args.len() != 1 {
            Err(InterpreterError::new_with_pos(
                "ENVIRON$ expected exactly one argument",
                function_name.location(),
            ))
        } else {
            let pos = args[0].location();
            match self.context_mut().demand_sub().pop_front_unnamed(pos)? {
                Variant::VString(env_var_name) => {
                    Ok(Variant::VString(self.stdlib.get_env_var(&env_var_name)))
                }
                _ => Err(InterpreterError::new_with_pos(
                    "Type mismatch at ENVIRON$",
                    pos,
                )),
            }
        }
    }

    pub fn run_built_in_function(&mut self, function_name: &Name, pos: Location) -> Result<()> {
        if function_name == &Name::from("ENVIRON$") {
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
        } else if function_name == &Name::from("_Undefined_%") {
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

impl InstructionGenerator {
    pub fn generate_built_in_function_call_instructions(
        &mut self,
        function_name: NameNode,
        args: Vec<ExpressionNode>,
    ) -> Result<()> {
        // TODO validate arg len for ENVIRON$
        let pos = function_name.location();
        self.generate_push_unnamed_args_instructions(args, pos)?;
        self.push(Instruction::PushStack, pos);
        self.push(
            Instruction::BuiltInFunction(function_name.strip_location()),
            pos,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::*;
    use crate::assert_has_variable;
    use crate::interpreter::{Stdlib, Variant};

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
