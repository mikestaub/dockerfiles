use super::{err, Instruction, InstructionGenerator, Result};
use crate::common::*;
use crate::linter::*;
use crate::variant::Variant;

impl InstructionGenerator {
    pub fn generate_expression_instructions(&mut self, e: ExpressionNode) -> Result<()> {
        self.do_generate_expression_instructions(e, false)
    }

    pub fn generate_const_expression_instructions(&mut self, e: ExpressionNode) -> Result<()> {
        self.do_generate_expression_instructions(e, true)
    }

    fn do_generate_expression_instructions(
        &mut self,
        e_node: ExpressionNode,
        only_const: bool,
    ) -> Result<()> {
        let (e, pos) = e_node.consume();
        match e {
            Expression::SingleLiteral(s) => {
                self.push(Instruction::Load(Variant::from(s)), pos);
                Ok(())
            }
            Expression::DoubleLiteral(s) => {
                self.push(Instruction::Load(Variant::from(s)), pos);
                Ok(())
            }
            Expression::StringLiteral(s) => {
                self.push(Instruction::Load(Variant::from(s)), pos);
                Ok(())
            }
            Expression::IntegerLiteral(s) => {
                self.push(Instruction::Load(Variant::from(s)), pos);
                Ok(())
            }
            Expression::LongLiteral(s) => {
                self.push(Instruction::Load(Variant::from(s)), pos);
                Ok(())
            }
            Expression::Variable(name) => {
                if !only_const {
                    self.push(Instruction::CopyVarToA(name), pos);
                    Ok(())
                } else {
                    // TODO this is probably caught by liter now
                    err("Invalid constant", pos)
                }
            }
            Expression::Constant(name) => {
                self.push(Instruction::CopyVarToA(name), pos);
                Ok(())
            }
            Expression::FunctionCall(n, args) => {
                if only_const {
                    err("Invalid constant", pos)
                } else {
                    let name_node = n.at(pos);
                    self.generate_function_call_instructions(name_node, args)?;
                    Ok(())
                }
            }
            Expression::BinaryExpression(op, left, right) => {
                self.push(Instruction::PushRegisters, pos);
                // TODO this implies right to left evaluation, double check with QBasic reference implementation
                self.do_generate_expression_instructions(*right, only_const)?;
                self.push(Instruction::CopyAToB, pos);
                self.do_generate_expression_instructions(*left, only_const)?;
                match op {
                    Operand::Plus => self.push(Instruction::Plus, pos),
                    Operand::Minus => self.push(Instruction::Minus, pos),
                    Operand::LessThan => self.push(Instruction::LessThan, pos),
                    Operand::LessOrEqualThan => self.push(Instruction::LessOrEqualThan, pos),
                }
                self.push(Instruction::PopRegisters, pos);
                Ok(())
            }
            Expression::UnaryExpression(op, child) => {
                match op {
                    UnaryOperand::Not => {
                        self.do_generate_expression_instructions(*child, only_const)?;
                        self.push(Instruction::NotA, pos);
                    }
                    UnaryOperand::Minus => {
                        self.do_generate_expression_instructions(*child, only_const)?;
                        self.push(Instruction::NegateA, pos);
                    }
                }
                Ok(())
            }
        }
    }
}
