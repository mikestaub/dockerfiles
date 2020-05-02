use super::{Instruction, InstructionNode, InterpreterError, Result, Variant};
use crate::common::*;
use crate::interpreter::function_context::FunctionContext;
use crate::interpreter::sub_context::SubContext;
use crate::interpreter::subprogram_resolver;
use crate::parser::*;

use std::collections::HashMap;

pub struct InstructionGenerator {
    pub instructions: Vec<InstructionNode>,
    pub constants: Vec<CaseInsensitiveString>,
    pub function_context: FunctionContext,
    pub sub_context: SubContext,
}

fn sanitize(original_program: ProgramNode) -> Result<(ProgramNode, FunctionContext, SubContext)> {
    subprogram_resolver::NoFunctionInConst::no_function_in_const(&original_program)?;
    subprogram_resolver::for_next_counter_match(&original_program)?;
    let (program, f_c, s_c) = subprogram_resolver::resolve(original_program)?;
    subprogram_resolver::AllSubsKnown::all_subs_known(&program, &s_c)?;
    subprogram_resolver::AllFunctionsKnown::all_functions_known(&program, &f_c)?;
    Ok((program, f_c, s_c))
}

pub fn generate_instructions(program: ProgramNode) -> Result<Vec<InstructionNode>> {
    let (p, f, s) = sanitize(program)?;
    let mut generator = InstructionGenerator::new(f, s);
    generator.generate_unresolved(p)?;
    generator.resolve_instructions()?;
    Ok(generator.instructions)
}

fn collect_labels(instructions: &Vec<InstructionNode>) -> HashMap<CaseInsensitiveString, usize> {
    let mut result: HashMap<CaseInsensitiveString, usize> = HashMap::new();
    for j in 0..instructions.len() {
        if let Instruction::Label(y) = instructions[j].as_ref() {
            result.insert(y.clone(), j);
        }
    }
    result
}

impl InstructionGenerator {
    pub fn new(function_context: FunctionContext, sub_context: SubContext) -> Self {
        Self {
            instructions: vec![],
            constants: vec![],
            function_context,
            sub_context,
        }
    }

    pub fn generate_unresolved(&mut self, program: ProgramNode) -> Result<()> {
        for t in program {
            let (top_level_token, pos) = t.consume();
            match top_level_token {
                TopLevelToken::Statement(s) => {
                    self.generate_statement_node_instructions(s.at(pos))?;
                }
                TopLevelToken::DefType(d) => {
                    self.push(Instruction::DefType(d), pos);
                }
                _ => unimplemented!(),
            }
        }

        // add HALT instruction at end of program to separate from the functions and subs
        // TODO: nice to have: use location of last statement
        self.push(Instruction::Halt, Location::start());

        // functions
        for x in self.function_context.implementations.clone().into_iter() {
            let (_, v) = x;
            let pos = v.location();
            let name = v.name;
            let block = v.block;
            let label = CaseInsensitiveString::new(format!(":fun:{}", name.bare_name()));
            self.push(Instruction::Label(label), pos);
            // set default value
            self.push(
                Instruction::Load(Variant::default_variant(name.qualifier())),
                pos,
            );
            self.push(Instruction::StoreAToResult, pos);
            self.generate_block_instructions(block)?;
            self.push(Instruction::PopRet, pos);
        }

        // subs
        for x in self.sub_context.implementations.clone().into_iter() {
            let (_, v) = x;
            let pos = v.location();
            let name = v.name;
            let block = v.block;
            let label = CaseInsensitiveString::new(format!(":sub:{}", name.bare_name()));
            self.push(Instruction::Label(label), pos);
            self.generate_block_instructions(block)?;
            self.push(Instruction::PopRet, pos);
        }

        Ok(())
    }

    pub fn resolve_instructions(&mut self) -> Result<()> {
        let labels = collect_labels(&self.instructions);
        // resolve jumps
        for instruction_node in self.instructions.iter_mut() {
            let instruction: &Instruction = instruction_node.as_ref();
            let pos: Location = instruction_node.location();
            if let Instruction::UnresolvedJump(x) = instruction {
                match labels.get(x) {
                    Some(idx) => {
                        *instruction_node = Instruction::Jump(*idx).at(pos);
                    }
                    None => {
                        return Err(InterpreterError::new_with_pos("Label not found", pos));
                    }
                }
            } else if let Instruction::UnresolvedJumpIfFalse(x) = instruction {
                match labels.get(x) {
                    Some(idx) => {
                        *instruction_node = Instruction::JumpIfFalse(*idx).at(pos);
                    }
                    None => {
                        return Err(InterpreterError::new_with_pos("Label not found", pos));
                    }
                }
            } else if let Instruction::SetUnresolvedErrorHandler(x) = instruction {
                match labels.get(x) {
                    Some(idx) => {
                        *instruction_node = Instruction::SetErrorHandler(*idx).at(pos);
                    }
                    None => {
                        return Err(InterpreterError::new_with_pos("Label not found", pos));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn push(&mut self, i: Instruction, pos: Location) {
        self.instructions.push(i.at(pos));
    }

    pub fn jump_if_false<S: AsRef<str>>(&mut self, prefix: S, pos: Location) {
        self.push(
            Instruction::UnresolvedJumpIfFalse(CaseInsensitiveString::new(format!(
                "_{}_{:?}",
                prefix.as_ref(),
                pos
            ))),
            pos,
        );
    }

    pub fn jump<S: AsRef<str>>(&mut self, prefix: S, pos: Location) {
        self.push(
            Instruction::UnresolvedJump(CaseInsensitiveString::new(format!(
                "_{}_{:?}",
                prefix.as_ref(),
                pos
            ))),
            pos,
        );
    }

    pub fn label<S: AsRef<str>>(&mut self, prefix: S, pos: Location) {
        self.push(
            Instruction::Label(CaseInsensitiveString::new(format!(
                "_{}_{:?}",
                prefix.as_ref(),
                pos
            ))),
            pos,
        );
    }

    pub fn store_temp_var<S: AsRef<str>>(&mut self, prefix: S, pos: Location) {
        self.push(
            Instruction::Store(Name::Bare(CaseInsensitiveString::new(format!(
                "{}{:?}",
                prefix.as_ref(),
                pos
            )))),
            pos,
        );
    }

    pub fn copy_temp_var_to_a<S: AsRef<str>>(&mut self, prefix: S, pos: Location) {
        self.push(
            Instruction::CopyVarToA(Name::Bare(CaseInsensitiveString::new(format!(
                "{}{:?}",
                prefix.as_ref(),
                pos
            )))),
            pos,
        );
    }

    pub fn copy_temp_var_to_b<S: AsRef<str>>(&mut self, prefix: S, pos: Location) {
        self.push(
            Instruction::CopyVarToB(Name::Bare(CaseInsensitiveString::new(format!(
                "{}{:?}",
                prefix.as_ref(),
                pos
            )))),
            pos,
        );
    }
}

// impl Visitor<QStatementNode> for Emitter {
//     fn visit(&mut self, a: &QStatementNode) -> Result<()> {
//         match a {
//             QStatementNode::Assignment(l, r) => {
//                 self.visit(r)?;
//                 self.push(Instruction::Store(l.as_ref().clone()), l.location());
//                 Ok(())
//             }
//             _ => unimplemented!(),
//         }
//     }
// }

// impl Visitor<QExpressionNode> for Emitter {
//     fn visit(&mut self, a: &QExpressionNode) -> Result<()> {
//         match a {

//         }
//     }
// }

// impl Visitor<QProgramNode> for Emitter {
//     fn visit(&mut self, a: &QProgramNode) -> Result<()> {
//         // first loop: top level statements
//         for x in a.iter() {
//             match x {
//                 QTopLevelTokenNode::Statement(s) => self.visit(s)?,
//                 _ => ()
//             }
//         }

//         // add HALT instruction at end of program to separate from the functions and subs
//         // TODO: nice to have: use location of last statement
//         self
//         .instructions
//         .push(Instruction::Halt.at(Location::new(1, 1)));

//         // then functions and subs
//         for x in a.iter() {
//             match x {
//                 QTopLevelTokenNode::FunctionImplementation(n, params, block, pos) => {
//                     self.visit(&(n, params, block, pos))?;
//                 }
//                 QTopLevelTokenNode::SubImplementation(n, params, block, pos) => {
//                     self.visit(&(n, params, block, pos))?;
//                 }
//             }
//         }
//         Ok(())
//     }
// }

// // function implementation
// impl Visitor<(&QNameNode, &Vec<QNameNode>, &QStatementNodes, &Location)> for Emitter {
//     fn visit(&mut self, f: &(&QNameNode, &Vec<QNameNode>, &QStatementNodes, &Location)) -> Result<()> {
//         let (name, params, block, pos) = *f;
//         let label = CaseInsensitiveString::new(format!(":fun:{}", name.bare_name()));
//         self.push(Instruction::Label(label), *pos);
//         // set default value
//         self.push(
//             Instruction::Load(Variant::default_variant(name.qualifier())), *pos
//         );
//         self
//             .push(Instruction::StoreAToResult, *pos);
//         self.visit(block)?;
//         self.push(Instruction::PopRet, *pos);
//         Ok(())
//     }
// }

// // sub implementation
// impl Visitor<(&BareNameNode, &Vec<QNameNode>, &QStatementNodes, &Location)> for Emitter {
//     fn visit(&mut self, f: &(&BareNameNode, &Vec<QNameNode>, &QStatementNodes, &Location)) -> Result<()> {
//         let (name, params, block, pos) = *f;
//         let label = CaseInsensitiveString::new(format!(":sub:{}", name.bare_name()));
//         self.push(Instruction::Label(label), *pos);
//         self.visit(block)?;
//         self.push(Instruction::PopRet, *pos);
//         Ok(())
//     }
// }

// impl PostVisitor<Vec<QStatementNode>> for Emitter {
//     fn post_visit(&mut self, a: &Vec<QStatementNode>) ->Result<()> { Ok(()) }
// }
