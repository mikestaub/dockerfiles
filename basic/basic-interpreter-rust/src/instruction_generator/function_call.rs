use super::instruction::*;
use super::{InstructionGenerator, Result};
use crate::common::*;
use crate::linter::*;

impl InstructionGenerator {
    pub fn generate_function_call_instructions(
        &mut self,
        function_name: QNameNode,
        args: Vec<ExpressionNode>,
    ) -> Result<()> {
        let pos = function_name.location();

        if is_built_in_function(function_name.as_ref().bare_name()) {
            self.generate_built_in_function_call_instructions(function_name, args)?;
        } else {
            let pos = function_name.location();
            let bare_name: &CaseInsensitiveString = function_name.bare_name();
            let function_impl = self.function_context.get_implementation(bare_name).unwrap();
            let label = CaseInsensitiveString::new(format!(":fun:{}", bare_name));

            self.generate_push_named_args_instructions(&function_impl.parameters, args, pos)?;
            self.push(Instruction::PushStack, pos);

            let idx = self.instructions.len();
            self.push(Instruction::PushRet(idx + 2), pos);
            self.push(Instruction::UnresolvedJump(label), pos);
        }
        self.push(Instruction::PopStack, pos);
        self.push(Instruction::CopyResultToA, pos);
        Ok(())
    }

    pub fn generate_push_named_args_instructions(
        &mut self,
        param_names: &Vec<QualifiedName>,
        expressions: Vec<ExpressionNode>,
        pos: Location,
    ) -> Result<()> {
        // TODO validate cast if by val, same type if by ref
        self.push(Instruction::PreparePush, pos);
        for (n, e_node) in param_names.iter().zip(expressions.into_iter()) {
            let (e, pos) = e_node.consume();
            match e {
                Expression::Variable(v_name) => {
                    self.push(
                        Instruction::SetNamedRefParam(NamedRefParam {
                            parameter_name: n.clone(),
                            argument_name: v_name,
                        }),
                        pos,
                    );
                }
                _ => {
                    self.generate_expression_instructions(e.at(pos))?;
                    self.push(Instruction::SetNamedValParam(n.clone()), pos);
                }
            }
        }
        Ok(())
    }

    pub fn generate_push_unnamed_args_instructions(
        &mut self,
        expressions: Vec<ExpressionNode>,
        pos: Location,
    ) -> Result<()> {
        self.push(Instruction::PreparePush, pos);
        for e_node in expressions.into_iter() {
            let (e, pos) = e_node.consume();
            match e {
                Expression::Variable(v_name) => {
                    self.push(Instruction::PushUnnamedRefParam(v_name), pos);
                }
                _ => {
                    self.generate_expression_instructions(e.at(pos))?;
                    self.push(Instruction::PushUnnamedValParam, pos);
                }
            }
        }
        Ok(())
    }
}
