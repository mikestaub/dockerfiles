use super::subprogram_context::*;
use crate::common::*;
use crate::linter::*;

pub fn resolve(program: ProgramNode) -> (ProgramNode, FunctionContext, SubContext) {
    let mut function_context = FunctionContext::new();
    let mut sub_context = SubContext::new();
    let mut reduced_program: ProgramNode = vec![];

    for top_level_token_node in program {
        let (top_level_token, pos) = top_level_token_node.consume();
        match top_level_token {
            TopLevelToken::FunctionImplementation(f) => {
                function_context.add_implementation(f.name, f.params, f.body, pos);
            }
            TopLevelToken::SubImplementation(s) => {
                sub_context.add_implementation(s.name, s.params, s.body, pos);
            }
            _ => reduced_program.push(top_level_token.at(pos)),
        }
    }
    (reduced_program, function_context, sub_context)
}
