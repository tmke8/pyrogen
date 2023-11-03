/*
 * parseTreeWalker.ts
 * Copyright (c) Microsoft Corporation.
 * Licensed under the MIT license.
 * Author: Eric Traut
 *
 * Class that traverses a parse tree.
 */

use crate::check::analyze::parseNodes::{
    from_boxed_expr, from_boxed_stmt, ArgumentNode, AssertNode, AssignmentExpressionNode,
    AssignmentNode, AugmentedAssignmentNode, AwaitNode, BinaryOperationNode, BreakNode, CallNode,
    CaseNode, ClassNode, ConstantNode, ContinueNode, DecoratorNode, DelNode,
    DictionaryExpandEntryNode, DictionaryKeyEntryNode, DictionaryNode, EllipsisNode, ErrorNode,
    ExceptNode, ForNode, FormatStringNode, FunctionAnnotationNode, FunctionNode, GlobalNode,
    IfNode, ImportAsNode, ImportFromAsNode, ImportFromNode, ImportNode, IndexNode, LambdaNode,
    ListComprehensionForNode, ListComprehensionIfNode, ListComprehensionNode, ListNode, MatchNode,
    MemberAccessNode, ModuleNameNode, ModuleNode, NameNode, NonlocalNode, NumberNode,
    ParameterNode, ParseNode, ParseNodeArray, ParseNodeType, PassNode, PatternAsNode,
    PatternCaptureNode, PatternClassArgumentNode, PatternClassNode, PatternLiteralNode,
    PatternMappingExpandEntryNode, PatternMappingKeyEntryNode, PatternMappingNode,
    PatternSequenceNode, PatternValueNode, RaiseNode, ReturnNode, SetNode, SliceNode,
    StatementListNode, StringListNode, StringNode, SuiteNode, TernaryNode, TryNode, TupleNode,
    TypeAliasNode, TypeAnnotationNode, TypeParameterListNode, TypeParameterNode,
    UnaryOperationNode, UnpackNode, WhileNode, WithItemNode, WithNode, YieldFromNode, YieldNode,
};

// Get child nodes of the given node.
pub fn getChildNodes(node: ParseNode) -> ParseNodeArray {
    match node {
        // ParseNode::Error => {
        //     vec![node.child, ...(node.decorators ?? [])]
        // }
        // ParseNode::Argument => {
        //     vec![node.name, node.valueExpression]
        // }
        ParseNode::Assert(node) => {
            vec![node.test, node.exceptionExpression]
        }
        ParseNode::AssignmentExpression(node) => {
            vec![node.target, node.value]
        }
        ParseNode::Assignment(node) => match node {
            AssignmentNode::Typed(node) => {
                vec![node.target, node.value, node.annotation]
            }
            AssignmentNode::Untyped(node) => {
                vec![node.targets, node.value]
            }
        },
        ParseNode::AugmentedAssignment(node) => {
            vec![node.target, node.value]
        }
        ParseNode::Await(node) => {
            vec![node.value]
        }
        ParseNode::BinaryOperation(node) => {
            vec![node.leftExpression, node.rightExpression]
        }
        ParseNode::Break(node) => {
            vec![]
        }
        // ParseNode::Call(node) => {
        //     vec![node.leftExpression, ...node.arguments]
        // }
        // ParseNode::Case(node) => {
        //     vec![node.pattern, node.guardExpression, node.suite]
        // }
        // ParseNode::Class(node) => {
        //     vec![...node.decorators, node.name, node.typeParameters, ...node.arguments, node.suite]
        // }
        ParseNode::Constant(node) => {
            vec![]
        }
        ParseNode::Continue(node) => {
            vec![]
        }
        // ParseNode::Decorator(node) => {
        //     vec![node.expression]
        // }
        ParseNode::Del(node) => node.expressions,
        ParseNode::Dictionary(node) => node.entries,
        // ParseNode::DictionaryExpandEntry(node) => {
        //     vec![node.expandExpression]
        // }
        // ParseNode::DictionaryKeyEntry(node) => {
        //     vec![node.keyExpression, node.valueExpression]
        // }
        ParseNode::Ellipsis(node) => {
            vec![]
        }
        ParseNode::If(node) => {
            vec![node.testExpression, node.ifSuite, node.elseSuite]
        }
        ParseNode::Import(node) => node.list,
        // ParseNode::ImportAs(node) => {
        //     vec![node.module, node.alias]
        // }
        // ParseNode::ImportFrom(node) => {
        //     vec![node.module, ...node.imports]
        // }
        // ParseNode::ImportFromAs(node) => {
        //     vec![node.name, node.alias]
        // }
        // ParseNode::Index(node) => {
        //     vec![node.baseExpression, ...node.items]
        // }
        // ParseNode::Except(node) => {
        //     vec![node.typeExpression, node.name, node.exceptSuite]
        // }
        ParseNode::For(node) => {
            vec![
                node.targetExpression,
                node.iterableExpression,
                node.forSuite,
                node.elseSuite,
            ]
        }
        // ParseNode::FormatString(node) => {
        //     vec![...node.fieldExpressions, ...(node.formatExpressions ?? [])]
        // }
        // ParseNode::Function(node) => {
        //     vec![
        //         ...node.decorators,
        //         node.name,
        //         node.typeParameters,
        //         ...node.parameters,
        //         node.returnTypeAnnotation,
        //         node.functionAnnotationComment,
        //         node.suite,
        //     ]
        // }
        // ParseNode::FunctionAnnotation(node) => {
        //     vec![...node.paramTypeAnnotations, node.returnTypeAnnotation]
        // }
        // ParseNode::Global(node) => node.nameList,
        // ParseNode::Lambda(node) => {
        //     vec![...node.parameters, node.expression]
        // }
        ParseNode::List(node) => node.elts,
        // ParseNode::ListComprehension(node) => {
        //     vec![node.expression, ...node.forIfNodes]
        // }
        ParseNode::ListComprehensionFor(node) => {
            vec![node.targetExpression, node.iterableExpression]
        }
        ParseNode::ListComprehensionIf(node) => {
            vec![node.testExpression]
        }
        // ParseNode::Match(node) => {
        //     vec![node.subjectExpression, ...node.cases]
        // }
        ParseNode::MemberAccess(node) => {
            vec![node.leftExpression, node.memberName]
        }
        ParseNode::ModuleName(node) => node.nameParts,
        // ParseNode::Module(node) => {
        //     vec![...node.statements]
        // }
        ParseNode::Name(node) => {
            vec![]
        }
        ParseNode::Nonlocal(node) => node.nameList,
        ParseNode::Number(node) => {
            vec![]
        }
        ParseNode::Parameter(node) => {
            vec![
                node.name,
                node.typeAnnotation,
                node.typeAnnotationComment,
                node.defaultValue,
            ]
        }
        ParseNode::Pass(node) => {
            vec![]
        }
        // ParseNode::PatternAs(node) => {
        //     vec![...node.orPatterns, node.target]
        // }
        // ParseNode::PatternClass(node) => {
        //     vec![node.className, ...node.arguments]
        // }
        ParseNode::PatternClassArgument(node) => {
            vec![node.name, node.pattern]
        }
        ParseNode::PatternCapture(node) => {
            vec![node.target]
        }
        ParseNode::PatternLiteral(node) => {
            vec![node.expression]
        }
        ParseNode::PatternMappingExpandEntry(node) => {
            vec![node.target]
        }
        ParseNode::PatternMappingKeyEntry(node) => {
            vec![node.keyPattern, node.valuePattern]
        }
        // ParseNode::PatternMapping(node) => {
        //     vec![...node.entries]
        // }
        // ParseNode::PatternSequence(node) => {
        //     vec![...node.entries]
        // }
        // ParseNode::PatternValue(node) => {
        //     vec![node.expression]
        // }
        ParseNode::Raise(node) => {
            vec![
                node.typeExpression,
                node.valueExpression,
                node.tracebackExpression,
            ]
        }
        ParseNode::Return(node) => {
            vec![node.value]
        }
        ParseNode::Set(node) => node.entries,
        ParseNode::Slice(node) => {
            vec![node.lower, node.upper, node.step]
        }
        ParseNode::StatementList(node) => node.statements,
        // ParseNode::StringList(node) => {
        //     vec![node.typeAnnotation, ...node.strings]
        // }
        ParseNode::String(node) => {
            vec![]
        }
        // ParseNode::Suite(node) => {
        //     vec![...node.statements]
        // }
        ParseNode::Ternary(node) => {
            vec![node.ifExpression, node.testExpression, node.elseExpression]
        }
        ParseNode::Tuple(node) => node.expressions,
        // ParseNode::Try(node) => {
        //     vec![node.trySuite, ...node.exceptClauses, node.elseSuite, node.finallySuite]
        // }
        ParseNode::TypeAlias(node) => {
            vec![
                Some(from_boxed_expr(node.name)),
                node.type_params,
                node.value,
            ]
        }
        // ParseNode::TypeAnnotation(node) => {
        //     vec![node.valueExpression, node.typeAnnotation]
        // }
        // ParseNode::TypeParameter(node) => {
        //     vec![node.name, node.boundExpression, node.defaultExpression]
        // }
        // ParseNode::TypeParameterList(node) => {
        //     vec![...node.parameters]
        // }
        ParseNode::UnaryOperation(node) => {
            vec![node.expression]
        }
        // ParseNode::Unpack(node) => {
        //     vec![node.expression]
        // }
        ParseNode::While(node) => {
            vec![
                Some(from_boxed_expr(node.test)),
                Some(from_boxed_expr(node.body)),
                Some(from_boxed_stmt(node.orelse)),
            ]
        }
        // ParseNode::With(node) => {
        //     vec![...node.withItems, node.suite]
        // }
        // ParseNode::WithItem(node) => {
        //     vec![node.expression, node.target]
        // }
        ParseNode::Yield(node) => {
            vec![node.value.map(|v| from_boxed_expr(v))]
        }
        ParseNode::YieldFrom(node) => {
            vec![Some(from_boxed_expr(node.value))]
        }
        _ => {
            // panic!("Unknown node type {}", node.nodeType);
        }
    }
}

// // To use this class, create a subclass and override the
// // visitXXX methods that you want to handle.
// export class ParseTreeVisitor<T> {
//     constructor(private readonly _default: T) {
//         // empty
//     }

//     visit(node: ParseNode): T {
//         switch (node.nodeType) {
//             case ParseNodeType.Error:
//                 return this.visitError(node);

//             case ParseNodeType.Argument:
//                 return this.visitArgument(node);

//             case ParseNodeType.Assert:
//                 return this.visitAssert(node);

//             case ParseNodeType.AssignmentExpression:
//                 return this.visitAssignmentExpression(node);

//             case ParseNodeType.Assignment:
//                 return this.visitAssignment(node);

//             case ParseNodeType.AugmentedAssignment:
//                 return this.visitAugmentedAssignment(node);

//             case ParseNodeType.Await:
//                 return this.visitAwait(node);

//             case ParseNodeType.BinaryOperation:
//                 return this.visitBinaryOperation(node);

//             case ParseNodeType.Break:
//                 return this.visitBreak(node);

//             case ParseNodeType.Call:
//                 return this.visitCall(node);

//             case ParseNodeType.Case:
//                 return this.visitCase(node);

//             case ParseNodeType.Class:
//                 return this.visitClass(node);

//             case ParseNodeType.Constant:
//                 return this.visitConstant(node);

//             case ParseNodeType.Continue:
//                 return this.visitContinue(node);

//             case ParseNodeType.Decorator:
//                 return this.visitDecorator(node);

//             case ParseNodeType.Del:
//                 return this.visitDel(node);

//             case ParseNodeType.Dictionary:
//                 return this.visitDictionary(node);

//             case ParseNodeType.DictionaryExpandEntry:
//                 return this.visitDictionaryExpandEntry(node);

//             case ParseNodeType.DictionaryKeyEntry:
//                 return this.visitDictionaryKeyEntry(node);

//             case ParseNodeType.Ellipsis:
//                 return this.visitEllipsis(node);

//             case ParseNodeType.If:
//                 return this.visitIf(node);

//             case ParseNodeType.Import:
//                 return this.visitImport(node);

//             case ParseNodeType.ImportAs:
//                 return this.visitImportAs(node);

//             case ParseNodeType.ImportFrom:
//                 return this.visitImportFrom(node);

//             case ParseNodeType.ImportFromAs:
//                 return this.visitImportFromAs(node);

//             case ParseNodeType.Index:
//                 return this.visitIndex(node);

//             case ParseNodeType.Except:
//                 return this.visitExcept(node);

//             case ParseNodeType.For:
//                 return this.visitFor(node);

//             case ParseNodeType.FormatString:
//                 return this.visitFormatString(node);

//             case ParseNodeType.Function:
//                 return this.visitFunction(node);

//             case ParseNodeType.FunctionAnnotation:
//                 return this.visitFunctionAnnotation(node);

//             case ParseNodeType.Global:
//                 return this.visitGlobal(node);

//             case ParseNodeType.Lambda:
//                 return this.visitLambda(node);

//             case ParseNodeType.List:
//                 return this.visitList(node);

//             case ParseNodeType.ListComprehension:
//                 return this.visitListComprehension(node);

//             case ParseNodeType.ListComprehensionFor:
//                 return this.visitListComprehensionFor(node);

//             case ParseNodeType.ListComprehensionIf:
//                 return this.visitListComprehensionIf(node);

//             case ParseNodeType.Match:
//                 return this.visitMatch(node);

//             case ParseNodeType.MemberAccess:
//                 return this.visitMemberAccess(node);

//             case ParseNodeType.ModuleName:
//                 return this.visitModuleName(node);

//             case ParseNodeType.Module:
//                 return this.visitModule(node);

//             case ParseNodeType.Name:
//                 return this.visitName(node);

//             case ParseNodeType.Nonlocal:
//                 return this.visitNonlocal(node);

//             case ParseNodeType.Number:
//                 return this.visitNumber(node);

//             case ParseNodeType.Parameter:
//                 return this.visitParameter(node);

//             case ParseNodeType.Pass:
//                 return this.visitPass(node);

//             case ParseNodeType.PatternAs:
//                 return this.visitPatternAs(node);

//             case ParseNodeType.PatternClass:
//                 return this.visitPatternClass(node);

//             case ParseNodeType.PatternClassArgument:
//                 return this.visitPatternClassArgument(node);

//             case ParseNodeType.PatternCapture:
//                 return this.visitPatternCapture(node);

//             case ParseNodeType.PatternLiteral:
//                 return this.visitPatternLiteral(node);

//             case ParseNodeType.PatternMappingExpandEntry:
//                 return this.visitPatternMappingExpandEntry(node);

//             case ParseNodeType.PatternMappingKeyEntry:
//                 return this.visitPatternMappingKeyEntry(node);

//             case ParseNodeType.PatternMapping:
//                 return this.visitPatternMapping(node);

//             case ParseNodeType.PatternSequence:
//                 return this.visitPatternSequence(node);

//             case ParseNodeType.PatternValue:
//                 return this.visitPatternValue(node);

//             case ParseNodeType.Raise:
//                 return this.visitRaise(node);

//             case ParseNodeType.Return:
//                 return this.visitReturn(node);

//             case ParseNodeType.Set:
//                 return this.visitSet(node);

//             case ParseNodeType.Slice:
//                 return this.visitSlice(node);

//             case ParseNodeType.StatementList:
//                 return this.visitStatementList(node);

//             case ParseNodeType.StringList:
//                 return this.visitStringList(node);

//             case ParseNodeType.String:
//                 return this.visitString(node);

//             case ParseNodeType.Suite:
//                 return this.visitSuite(node);

//             case ParseNodeType.Ternary:
//                 return this.visitTernary(node);

//             case ParseNodeType.Tuple:
//                 return this.visitTuple(node);

//             case ParseNodeType.Try:
//                 return this.visitTry(node);

//             case ParseNodeType.TypeAlias:
//                 return this.visitTypeAlias(node);

//             case ParseNodeType.TypeAnnotation:
//                 return this.visitTypeAnnotation(node);

//             case ParseNodeType.TypeParameter:
//                 return this.visitTypeParameter(node);

//             case ParseNodeType.TypeParameterList:
//                 return this.visitTypeParameterList(node);

//             case ParseNodeType.UnaryOperation:
//                 return this.visitUnaryOperation(node);

//             case ParseNodeType.Unpack:
//                 return this.visitUnpack(node);

//             case ParseNodeType.While:
//                 return this.visitWhile(node);

//             case ParseNodeType.With:
//                 return this.visitWith(node);

//             case ParseNodeType.WithItem:
//                 return this.visitWithItem(node);

//             case ParseNodeType.Yield:
//                 return this.visitYield(node);

//             case ParseNodeType.YieldFrom:
//                 return this.visitYieldFrom(node);

//             default:
//                 debug.assertNever(node, `Unknown node type ${node}`);
//         }
//     }

//     // Override these methods as necessary.
//     visitArgument(node: ArgumentNode) {
//         return this._default;
//     }

//     visitAssert(node: AssertNode) {
//         return this._default;
//     }

//     visitAssignment(node: AssignmentNode) {
//         return this._default;
//     }

//     visitAssignmentExpression(node: AssignmentExpressionNode) {
//         return this._default;
//     }

//     visitAugmentedAssignment(node: AugmentedAssignmentNode) {
//         return this._default;
//     }

//     visitAwait(node: AwaitNode) {
//         return this._default;
//     }

//     visitBinaryOperation(node: BinaryOperationNode) {
//         return this._default;
//     }

//     visitBreak(node: BreakNode) {
//         return this._default;
//     }

//     visitCall(node: CallNode) {
//         return this._default;
//     }

//     visitCase(node: CaseNode) {
//         return this._default;
//     }

//     visitClass(node: ClassNode) {
//         return this._default;
//     }

//     visitTernary(node: TernaryNode) {
//         return this._default;
//     }

//     visitContinue(node: ContinueNode) {
//         return this._default;
//     }

//     visitConstant(node: ConstantNode) {
//         return this._default;
//     }

//     visitDecorator(node: DecoratorNode) {
//         return this._default;
//     }

//     visitDel(node: DelNode) {
//         return this._default;
//     }

//     visitDictionary(node: DictionaryNode) {
//         return this._default;
//     }

//     visitDictionaryKeyEntry(node: DictionaryKeyEntryNode) {
//         return this._default;
//     }

//     visitDictionaryExpandEntry(node: DictionaryExpandEntryNode) {
//         return this._default;
//     }

//     visitError(node: ErrorNode) {
//         return this._default;
//     }

//     visitEllipsis(node: EllipsisNode) {
//         return this._default;
//     }

//     visitIf(node: IfNode) {
//         return this._default;
//     }

//     visitImport(node: ImportNode) {
//         return this._default;
//     }

//     visitImportAs(node: ImportAsNode) {
//         return this._default;
//     }

//     visitImportFrom(node: ImportFromNode) {
//         return this._default;
//     }

//     visitImportFromAs(node: ImportFromAsNode) {
//         return this._default;
//     }

//     visitIndex(node: IndexNode) {
//         return this._default;
//     }

//     visitExcept(node: ExceptNode) {
//         return this._default;
//     }

//     visitFor(node: ForNode) {
//         return this._default;
//     }

//     visitFormatString(node: FormatStringNode) {
//         return this._default;
//     }

//     visitFunction(node: FunctionNode) {
//         return this._default;
//     }

//     visitFunctionAnnotation(node: FunctionAnnotationNode) {
//         return this._default;
//     }

//     visitGlobal(node: GlobalNode) {
//         return this._default;
//     }

//     visitLambda(node: LambdaNode) {
//         return this._default;
//     }

//     visitList(node: ListNode) {
//         return this._default;
//     }

//     visitListComprehension(node: ListComprehensionNode) {
//         return this._default;
//     }

//     visitListComprehensionFor(node: ListComprehensionForNode) {
//         return this._default;
//     }

//     visitListComprehensionIf(node: ListComprehensionIfNode) {
//         return this._default;
//     }

//     visitMatch(node: MatchNode) {
//         return this._default;
//     }

//     visitMemberAccess(node: MemberAccessNode) {
//         return this._default;
//     }

//     visitModule(node: ModuleNode) {
//         return this._default;
//     }

//     visitModuleName(node: ModuleNameNode) {
//         return this._default;
//     }

//     visitName(node: NameNode) {
//         return this._default;
//     }

//     visitNonlocal(node: NonlocalNode) {
//         return this._default;
//     }

//     visitNumber(node: NumberNode) {
//         return this._default;
//     }

//     visitParameter(node: ParameterNode) {
//         return this._default;
//     }

//     visitPass(node: PassNode) {
//         return this._default;
//     }

//     visitPatternCapture(node: PatternCaptureNode) {
//         return this._default;
//     }

//     visitPatternClass(node: PatternClassNode) {
//         return this._default;
//     }

//     visitPatternClassArgument(node: PatternClassArgumentNode) {
//         return this._default;
//     }

//     visitPatternAs(node: PatternAsNode) {
//         return this._default;
//     }

//     visitPatternLiteral(node: PatternLiteralNode) {
//         return this._default;
//     }

//     visitPatternMappingExpandEntry(node: PatternMappingExpandEntryNode) {
//         return this._default;
//     }

//     visitPatternSequence(node: PatternSequenceNode) {
//         return this._default;
//     }

//     visitPatternValue(node: PatternValueNode) {
//         return this._default;
//     }

//     visitPatternMappingKeyEntry(node: PatternMappingKeyEntryNode) {
//         return this._default;
//     }

//     visitPatternMapping(node: PatternMappingNode) {
//         return this._default;
//     }

//     visitRaise(node: RaiseNode) {
//         return this._default;
//     }

//     visitReturn(node: ReturnNode) {
//         return this._default;
//     }

//     visitSet(node: SetNode) {
//         return this._default;
//     }

//     visitSlice(node: SliceNode) {
//         return this._default;
//     }

//     visitStatementList(node: StatementListNode) {
//         return this._default;
//     }

//     visitString(node: StringNode) {
//         return this._default;
//     }

//     visitStringList(node: StringListNode) {
//         return this._default;
//     }

//     visitSuite(node: SuiteNode) {
//         return this._default;
//     }

//     visitTuple(node: TupleNode) {
//         return this._default;
//     }

//     visitTry(node: TryNode) {
//         return this._default;
//     }

//     visitTypeAlias(node: TypeAliasNode) {
//         return this._default;
//     }

//     visitTypeAnnotation(node: TypeAnnotationNode) {
//         return this._default;
//     }

//     visitTypeParameter(node: TypeParameterNode) {
//         return this._default;
//     }

//     visitTypeParameterList(node: TypeParameterListNode) {
//         return this._default;
//     }

//     visitUnaryOperation(node: UnaryOperationNode) {
//         return this._default;
//     }

//     visitUnpack(node: UnpackNode) {
//         return this._default;
//     }

//     visitWhile(node: WhileNode) {
//         return this._default;
//     }

//     visitWith(node: WithNode) {
//         return this._default;
//     }

//     visitWithItem(node: WithItemNode) {
//         return this._default;
//     }

//     visitYield(node: YieldNode) {
//         return this._default;
//     }

//     visitYieldFrom(node: YieldFromNode) {
//         return this._default;
//     }
// }

// // To use this class, create a subclass and override the
// // visitXXX methods that you want to handle.
// export class ParseTreeWalker extends ParseTreeVisitor<boolean> {
//     constructor() {
//         super(/* default */ true);
//     }

//     walk(node: ParseNode): void {
//         const childrenToWalk = this.visitNode(node);
//         if (childrenToWalk.length > 0) {
//             this.walkMultiple(childrenToWalk);
//         }
//     }

//     walkMultiple(nodes: ParseNodeArray) {
//         nodes.forEach((node) => {
//             if (node) {
//                 this.walk(node);
//             }
//         });
//     }

//     // If this.visit(node) returns true, all child nodes for the node are returned.
//     // If the method returns false, we assume that the handler has already handled the
//     // child nodes, so an empty list is returned.
//     visitNode(node: ParseNode): ParseNodeArray {
//         return this.visit(node) ? getChildNodes(node) : [];
//     }
// }
