use super::error::*;
use super::instruction::*;
use super::subprogram_context::*;
use super::subprogram_resolver;
use crate::common::*;
use crate::parser::*;
use crate::variant::Variant;

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
                        return err("Label not found", pos);
                    }
                }
            } else if let Instruction::UnresolvedJumpIfFalse(x) = instruction {
                match labels.get(x) {
                    Some(idx) => {
                        *instruction_node = Instruction::JumpIfFalse(*idx).at(pos);
                    }
                    None => {
                        return err("Label not found", pos);
                    }
                }
            } else if let Instruction::SetUnresolvedErrorHandler(x) = instruction {
                match labels.get(x) {
                    Some(idx) => {
                        *instruction_node = Instruction::SetErrorHandler(*idx).at(pos);
                    }
                    None => {
                        return err("Label not found", pos);
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

    pub fn generate_assignment_instructions(
        &mut self,
        l: NameNode,
        r: ExpressionNode,
    ) -> Result<()> {
        self.generate_expression_instructions(r)?;
        let pos = l.location();
        self.push(Instruction::Store(l.strip_location()), pos);
        Ok(())
    }
}
