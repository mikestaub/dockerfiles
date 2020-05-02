use crate::common::*;
use crate::interpreter::context::Argument;
use crate::interpreter::context_owner::ContextOwner;
use crate::interpreter::{
    Instruction, InstructionContext, Interpreter, InterpreterError, Result, Stdlib, Variant,
};
use crate::parser::{BareNameNode, ExpressionNode, QualifiedName, TypeQualifier, TypeResolver};

pub fn is_built_in_sub(sub_name: &BareNameNode) -> bool {
    sub_name.as_ref() == "ENVIRON"
        || sub_name.as_ref() == "PRINT"
        || sub_name.as_ref() == "INPUT"
        || sub_name.as_ref() == "SYSTEM"
}

impl<S: Stdlib> Interpreter<S> {
    pub fn generate_built_in_sub_call_instructions(
        &self,
        result: &mut InstructionContext,
        name_node: BareNameNode,
        args: Vec<ExpressionNode>,
    ) -> Result<()> {
        let (name, pos) = name_node.consume();
        if &name == "SYSTEM" {
            // TODO ensure no args
            result.instructions.push(Instruction::Halt.at(pos));
        } else {
            self.generate_push_unnamed_args_instructions(result, args, pos)?;
            result.instructions.push(Instruction::PushStack.at(pos));
            result
                .instructions
                .push(Instruction::BuiltInSub(name).at(pos));
        }
        Ok(())
    }

    pub fn run_built_in_sub(&mut self, name: &CaseInsensitiveString, pos: Location) -> Result<()> {
        if name == "PRINT" {
            let mut print_args: Vec<String> = vec![];
            loop {
                match self.context_mut().demand_sub().try_pop_front_unnamed(pos)? {
                    Some(v) => print_args.push(v.to_string()),
                    None => {
                        break;
                    }
                }
            }
            self.stdlib.print(print_args);
            Ok(())
        } else if name == "ENVIRON" {
            self._do_environ_sub(
                // TODO cleanup
                &name.clone().at(pos),
                &vec![ExpressionNode::StringLiteral("".to_string(), pos)],
            )
        } else if name == "INPUT" {
            self._do_input(pos)
        } else if name == "SYSTEM" {
            // TODO: remove after migrated to ng
            self.stdlib.system();
            Ok(())
        } else {
            panic!("Unknown sub {}", name)
        }
    }

    fn _do_environ_sub(
        &mut self,
        sub_name_node: &BareNameNode,
        args: &Vec<ExpressionNode>,
    ) -> Result<()> {
        match self
            .context_mut()
            .demand_sub()
            .pop_front_unnamed(sub_name_node.location())?
        {
            Variant::VString(arg_string_value) => {
                let parts: Vec<&str> = arg_string_value.split("=").collect();
                if parts.len() != 2 {
                    Err(InterpreterError::new_with_pos(
                        "Invalid expression. Must be name=value.",
                        args[0].location(),
                    ))
                } else {
                    self.stdlib
                        .set_env_var(parts[0].to_string(), parts[1].to_string());
                    Ok(())
                }
            }
            _ => Err(InterpreterError::new_with_pos(
                "Type mismatch",
                args[0].location(),
            )),
        }
    }

    fn _do_input(&mut self, pos: Location) -> Result<()> {
        loop {
            match &self.context_mut().demand_sub().pop_front_unnamed_arg() {
                Some(a) => match a {
                    Argument::ByRef(n) => {
                        self._do_input_one_var(a, n, pos)?;
                    }
                    _ => {
                        return Err(InterpreterError::new_with_pos("Expected variable", pos));
                    }
                },
                None => {
                    break;
                }
            }
        }
        Ok(())
    }

    fn _do_input_one_var(&mut self, a: &Argument, n: &QualifiedName, pos: Location) -> Result<()> {
        let raw_input: String = self
            .stdlib
            .input()
            .map_err(|e| InterpreterError::new_with_pos(e.to_string(), pos))?;
        let q: TypeQualifier = self.type_resolver.resolve(n);
        let variable_value = match q {
            TypeQualifier::BangSingle => Variant::from(
                parse_single_input(raw_input)
                    .map_err(|e| InterpreterError::new_with_pos(e, pos))?,
            ),
            TypeQualifier::DollarString => Variant::from(raw_input),
            TypeQualifier::PercentInteger => Variant::from(
                parse_int_input(raw_input).map_err(|e| InterpreterError::new_with_pos(e, pos))?,
            ),
            _ => unimplemented!(),
        };
        self.context_mut()
            .demand_sub()
            .set_value_to_popped_arg(a, variable_value, pos)
    }
}

fn parse_single_input(s: String) -> std::result::Result<f32, String> {
    if s.is_empty() {
        Ok(0.0)
    } else {
        s.parse::<f32>()
            .map_err(|e| format!("Could not parse {} as float: {}", s, e))
    }
}

fn parse_int_input(s: String) -> std::result::Result<i32, String> {
    if s.is_empty() {
        Ok(0)
    } else {
        s.parse::<i32>()
            .map_err(|e| format!("Could not parse {} as int: {}", s, e))
    }
}
