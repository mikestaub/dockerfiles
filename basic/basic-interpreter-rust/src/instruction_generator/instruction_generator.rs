use super::error::Result;
use super::instruction::*;
use super::subprogram_context::*;
use super::subprogram_resolver;
use crate::common::*;
use crate::linter::*;
use crate::variant::Variant;

use std::collections::HashMap;

pub struct InstructionGenerator {
    pub instructions: Vec<InstructionNode>,
    pub function_context: FunctionContext,
    pub sub_context: SubContext,
}

pub fn generate_instructions(program: ProgramNode) -> Result<Vec<InstructionNode>> {
    // TODO convert to a different return type, not ProgramNode
    let (p, f, s) = subprogram_resolver::resolve(program);
    let mut generator = InstructionGenerator::new(f, s);
    generator.generate_unresolved(p)?;
    generator.resolve_instructions();
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
                _ => unimplemented!(),
            }
        }

        // add HALT instruction at end of program to separate from the functions and subs
        self.push(
            Instruction::Halt,
            Location::new(std::u32::MAX, std::u32::MAX),
        );

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

    pub fn resolve_instructions(&mut self) {
        let labels = collect_labels(&self.instructions);
        // resolve jumps
        for instruction_node in self.instructions.iter_mut() {
            let instruction: &Instruction = instruction_node.as_ref();
            let pos: Location = instruction_node.location();
            if let Instruction::UnresolvedJump(x) = instruction {
                *instruction_node = Instruction::Jump(*labels.get(x).unwrap()).at(pos);
            } else if let Instruction::UnresolvedJumpIfFalse(x) = instruction {
                *instruction_node = Instruction::JumpIfFalse(*labels.get(x).unwrap()).at(pos);
            } else if let Instruction::SetUnresolvedErrorHandler(x) = instruction {
                *instruction_node = Instruction::SetErrorHandler(*labels.get(x).unwrap()).at(pos);
            }
        }
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

    pub fn generate_assignment_instructions(
        &mut self,
        l: QNameNode,
        r: ExpressionNode,
    ) -> Result<()> {
        self.generate_expression_instructions(r)?;
        let pos = l.location();
        self.push(Instruction::Store(l.strip_location()), pos);
        Ok(())
    }
}
