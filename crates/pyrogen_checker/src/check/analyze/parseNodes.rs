/*
 * parseNodes.ts
 * Copyright (c) Microsoft Corporation.
 * Licensed under the MIT license.
 * Author: Eric Traut
 *
 * Class that traverses a parse tree.
 */

use rustpython_ast::{
    bigint::BigInt, text_size::TextRange, Arguments, Constant, Expr, ExprAttribute, ExprAwait,
    ExprBinOp, ExprCall, ExprConstant, ExprDict, ExprIfExp, ExprJoinedStr, ExprLambda, ExprList,
    ExprListComp, ExprName, ExprNamedExpr, ExprSet, ExprSlice, ExprStarred, ExprSubscript,
    ExprTuple, ExprUnaryOp, ExprYield, ExprYieldFrom, Mod, Stmt, StmtAnnAssign, StmtAssert,
    StmtAssign, StmtAugAssign, StmtBreak, StmtClassDef, StmtContinue, StmtDelete, StmtFor,
    StmtFunctionDef, StmtGlobal, StmtIf, StmtImport, StmtImportFrom, StmtNonlocal, StmtPass,
    StmtRaise, StmtReturn, StmtTry, StmtTypeAlias, StmtWhile, StmtWith,
};

pub type ArgumentNode = Arguments;
pub type AssertNode = StmtAssert;
pub type AssignmentExpressionNode = ExprNamedExpr;
pub enum AssignmentNode {
    Untyped(StmtAssign),
    Typed(StmtAnnAssign),
}
pub type AugmentedAssignmentNode = StmtAugAssign;
pub type AwaitNode = ExprAwait;
pub type BinaryOperationNode = ExprBinOp;
pub type BreakNode = StmtBreak;
pub type CallNode = ExprCall;
// pub type CaseNode =
pub type ClassNode = StmtClassDef;
pub struct ConstantNode {
    pub value: ConstantNodeValue,
    pub range: TextRange,
}
pub enum ConstantNodeValue {
    None,
    True,
    False,
}
pub type ContinueNode = StmtContinue;
pub struct DecoratorNode(Expr<TextRange>);
pub type DelNode = StmtDelete;
// pub type DictionaryExpandEntryNode =
// pub type DictionaryKeyEntryNode =
pub type DictionaryNode = ExprDict;
pub struct EllipsisNode {
    pub range: TextRange,
}
// pub type ErrorNode =
// pub type ExceptNode =
pub type FormatStringNode = ExprJoinedStr;
pub type ForNode = StmtFor;
// pub type FunctionAnnotationNode =
pub type FunctionNode = StmtFunctionDef;
pub type GlobalNode = StmtGlobal;
pub type IfNode = StmtIf;
// pub type ImportAsNode // part of StmtImport
// pub type ImportFromAsNode // part of StmtImportFrom
pub type ImportFromNode = StmtImportFrom;
pub type ImportNode = StmtImport;
pub type IndexNode = ExprSubscript;
pub type LambdaNode = ExprLambda;
// pub type ListComprehensionForNode =
// pub type ListComprehensionIfNode =
pub type ListComprehensionNode = ExprListComp;
pub type ListNode = ExprList;
// pub type MatchNode =
pub type MemberAccessNode = ExprAttribute;
// pub type ModuleNameNode =
pub type ModuleNode = Mod;
pub type NameNode = ExprName;
pub type NonlocalNode = StmtNonlocal;
// pub type NumberNode // part of ExprConstant
pub struct NumberNode {
    pub value: NumberNodeValue,
    pub range: TextRange,
}
pub enum NumberNodeValue {
    Int(BigInt),
    Float(f64),
    Complex { real: f64, imag: f64 },
}
// pub type ParameterNode =
// pub type ParseNode =
// pub type ParseNodeArray =
// pub type ParseNodeType =
pub type PassNode = StmtPass;
// pub type PatternAsNode =
// pub type PatternCaptureNode =
// pub type PatternClassArgumentNode =
// pub type PatternClassNode =
// pub type PatternLiteralNode =
// pub type PatternMappingExpandEntryNode =
// pub type PatternMappingKeyEntryNode =
// pub type PatternMappingNode =
// pub type PatternSequenceNode =
// pub type PatternValueNode =
pub type RaiseNode = StmtRaise;
pub type ReturnNode = StmtReturn;
pub type SetNode = ExprSet;
pub type SliceNode = ExprSlice;
// pub type StatementListNode =
// pub type StringListNode =
// pub type StringNode // part of ExprConstant
pub struct StringNode {
    pub value: String,
    pub range: TextRange,
}
// pub type SuiteNode =
pub type TernaryNode = ExprIfExp;
pub type TryNode = StmtTry;
pub type TupleNode = ExprTuple;
pub type TypeAliasNode = StmtTypeAlias;
// pub type TypeAnnotationNode =
// pub type TypeParameterListNode =
// pub type TypeParameterNode =
pub type UnaryOperationNode = ExprUnaryOp;
pub type UnpackNode = ExprStarred;
pub type WhileNode = StmtWhile;
// pub type WithItemNode =
pub type WithNode = StmtWith;
pub type YieldFromNode = ExprYieldFrom;
pub type YieldNode = ExprYield;

pub enum ParseNode {
    // ErrorNode(ErrorNode),
    Argument(ArgumentNode),
    Assert(AssertNode),
    AssignmentExpression(AssignmentExpressionNode),
    Assignment(AssignmentNode),
    AugmentedAssignment(AugmentedAssignmentNode),
    Await(AwaitNode),
    BinaryOperation(BinaryOperationNode),
    Break(BreakNode),
    Call(CallNode),
    // Case(CaseNode),
    Class(ClassNode),
    Constant(ConstantNode),
    Continue(ContinueNode),
    Decorator(DecoratorNode),
    Del(DelNode),
    Dictionary(DictionaryNode),
    // DictionaryExpandEntry(DictionaryExpandEntryNode),
    // DictionaryKeyEntry(DictionaryKeyEntryNode),
    Ellipsis(EllipsisNode),
    If(IfNode),
    Import(ImportNode),
    // ImportAs(ImportAsNode),
    ImportFrom(ImportFromNode),
    // ImportFromAs(ImportFromAsNode),
    Index(IndexNode),
    // Except(ExceptNode),
    For(ForNode),
    FormatString(FormatStringNode),
    Function(FunctionNode),
    // FunctionAnnotation(FunctionAnnotationNode),
    Global(GlobalNode),
    Lambda(LambdaNode),
    List(ListNode),
    ListComprehension(ListComprehensionNode),
    // ListComprehensionFor(ListComprehensionForNode),
    // ListComprehensionIf(ListComprehensionIfNode),
    // Match(MatchNode),
    MemberAccess(MemberAccessNode),
    // ModuleName(ModuleNameNode),
    Module(ModuleNode),
    Name(NameNode),
    Nonlocal(NonlocalNode),
    Number(NumberNode),
    // Parameter(ParameterNode),
    Pass(PassNode),
    // PatternAs(PatternAsNode),
    // PatternClass(PatternClassNode),
    // PatternClassArgument(PatternClassArgumentNode),
    // PatternCapture(PatternCaptureNode),
    // PatternLiteral(PatternLiteralNode),
    // PatternMappingExpandEntry(PatternMappingExpandEntryNode),
    // PatternMappingKeyEntry(PatternMappingKeyEntryNode),
    // PatternMapping(PatternMappingNode),
    // PatternSequence(PatternSequenceNode),
    // PatternValue(PatternValueNode),
    Raise(RaiseNode),
    Return(ReturnNode),
    Set(SetNode),
    Slice(SliceNode),
    // StatementList(StatementListNode),
    // StringList(StringListNode),
    String(StringNode),
    // Suite(SuiteNode),
    Ternary(TernaryNode),
    Tuple(TupleNode),
    Try(TryNode),
    TypeAlias(TypeAliasNode),
    // TypeAnnotation(TypeAnnotationNode),
    // TypeParameter(TypeParameterNode),
    // TypeParameterList(TypeParameterListNode),
    UnaryOperation(UnaryOperationNode),
    Unpack(UnpackNode),
    While(WhileNode),
    With(WithNode),
    // WithItem(WithItemNode),
    Yield(YieldNode),
    YieldFrom(YieldFromNode),
}

impl From<Expr<TextRange>> for ParseNode {
    fn from(value: Expr<TextRange>) -> Self {
        match value {
            // Expr::BoolOp(expr) => ParseNode::(expr),  // needs to be merged into binary op
            Expr::NamedExpr(expr) => ParseNode::AssignmentExpression(expr),
            Expr::BinOp(expr) => ParseNode::BinaryOperation(expr),
            Expr::UnaryOp(expr) => ParseNode::UnaryOperation(expr),
            Expr::Lambda(expr) => ParseNode::Lambda(expr),
            Expr::IfExp(expr) => ParseNode::Ternary(expr),
            Expr::Dict(expr) => ParseNode::Dictionary(expr),
            Expr::Set(expr) => ParseNode::Set(expr),
            Expr::ListComp(expr) => ParseNode::ListComprehension(expr),
            // Expr::SetComp(expr) => ParseNode::(expr),
            // Expr::DictComp(expr) => ParseNode::(expr),
            // Expr::GeneratorExp(expr) => ParseNode::(expr),
            Expr::Await(expr) => ParseNode::Await(expr),
            Expr::Yield(expr) => ParseNode::Yield(expr),
            Expr::YieldFrom(expr) => ParseNode::YieldFrom(expr),
            // Expr::Compare(expr) => ParseNode::(expr),
            Expr::Call(expr) => ParseNode::Call(expr),
            // Expr::FormattedValue(expr) => ParseNode::(expr),
            Expr::JoinedStr(expr) => ParseNode::FormatString(expr),
            Expr::Constant(expr) => match expr.value {
                Constant::Str(value) => ParseNode::String(StringNode {
                    value,
                    range: expr.range,
                }),
                Constant::Complex { real, imag } => ParseNode::Number(NumberNode {
                    value: NumberNodeValue::Complex { real, imag },
                    range: expr.range,
                }),
                Constant::Float(value) => ParseNode::Number(NumberNode {
                    value: NumberNodeValue::Float(value),
                    range: expr.range,
                }),
                Constant::Int(value) => ParseNode::Number(NumberNode {
                    value: NumberNodeValue::Int(value),
                    range: expr.range,
                }),
                Constant::Ellipsis => ParseNode::Ellipsis(EllipsisNode { range: expr.range }),
                Constant::Bool(value) => ParseNode::Constant(ConstantNode {
                    value: if value {
                        ConstantNodeValue::True
                    } else {
                        ConstantNodeValue::False
                    },
                    range: expr.range,
                }),
                Constant::None => ParseNode::Constant(ConstantNode {
                    value: ConstantNodeValue::None,
                    range: expr.range,
                }),
                Constant::Tuple(values) => ParseNode::Tuple(ExprTuple {
                    elts: values
                        .into_iter()
                        .map(|value| Expr::Constant(ExprConstant { value, kind: None }))
                        .collect(),
                    range: expr.range,
                }),
            },
            Expr::Attribute(expr) => ParseNode::MemberAccess(expr),
            Expr::Subscript(expr) => ParseNode::Index(expr),
            Expr::Starred(expr) => ParseNode::Unpack(expr),
            Expr::Name(expr) => ParseNode::Name(expr),
            Expr::List(expr) => ParseNode::List(expr),
            Expr::Tuple(expr) => ParseNode::Tuple(expr),
            Expr::Slice(expr) => ParseNode::Slice(expr),
        }
    }
}

impl From<Stmt<TextRange>> for ParseNode {
    fn from(stmt: Stmt<TextRange>) -> ParseNode {
        match stmt {
            Stmt::FunctionDef(stmt) => ParseNode::Function(stmt),
            // Stmt::AsyncFunctionDef(stmt) => ParseNode::(stmt),
            Stmt::ClassDef(stmt) => ParseNode::Class(stmt),
            Stmt::Return(stmt) => ParseNode::Return(stmt),
            Stmt::Delete(stmt) => ParseNode::Del(stmt),
            Stmt::Assign(stmt) => ParseNode::Assignment(AssignmentNode::Untyped(stmt)),
            Stmt::TypeAlias(stmt) => ParseNode::TypeAlias(stmt),
            Stmt::AugAssign(stmt) => ParseNode::AugmentedAssignment(stmt),
            Stmt::AnnAssign(stmt) => ParseNode::Assignment(AssignmentNode::Typed(stmt)),
            Stmt::For(stmt) => ParseNode::For(stmt),
            // Stmt::AsyncFor(stmt) => ParseNode::(stmt),
            Stmt::While(stmt) => ParseNode::While(stmt),
            Stmt::If(stmt) => ParseNode::If(stmt),
            Stmt::With(stmt) => ParseNode::With(stmt),
            // Stmt::AsyncWith(stmt) => ParseNode::(stmt),
            // Stmt::Match(stmt) => ParseNode::(stmt),
            Stmt::Raise(stmt) => ParseNode::Raise(stmt),
            Stmt::Try(stmt) => ParseNode::Try(stmt),
            // Stmt::TryStar(stmt) => ParseNode::(stmt),
            Stmt::Assert(stmt) => ParseNode::Assert(stmt),
            Stmt::Import(stmt) => ParseNode::Import(stmt),
            Stmt::ImportFrom(stmt) => ParseNode::ImportFrom(stmt),
            Stmt::Global(stmt) => ParseNode::Global(stmt),
            Stmt::Nonlocal(stmt) => ParseNode::Nonlocal(stmt),
            // Stmt::Expr(stmt) => ParseNode::(stmt),
            Stmt::Pass(stmt) => ParseNode::Pass(stmt),
            Stmt::Break(stmt) => ParseNode::Break(stmt),
            Stmt::Continue(stmt) => ParseNode::Continue(stmt),
        }
    }
}

#[inline]
pub const fn from_boxed_expr(expr: Box<Expr<TextRange>>) -> Box<ParseNode> {
    Box::new(ParseNode::from(*expr))
}

#[inline]
pub const fn from_boxed_stmt(stmt: Box<Stmt<TextRange>>) -> Box<ParseNode> {
    Box::new(ParseNode::from(*stmt))
}

pub enum EvaluationScopeNode {
    LambdaNode(LambdaNode),
    FunctionNode(FunctionNode),
    ModuleNode(ModuleNode),
    ClassNode(ClassNode),
    ListComprehensionNode(ListComprehensionNode),
}
pub enum ExecutionScopeNode {
    LambdaNode(LambdaNode),
    FunctionNode(FunctionNode),
    ModuleNode(ModuleNode),
}
pub enum TypeParameterScopeNode {
    FunctionNode(FunctionNode),
    ClassNode(ClassNode),
}

pub type ParseNodeArray = Vec<Option<Box<ParseNode>>>;
