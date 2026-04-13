use crate::diagnostics::SourceSpan;

pub type NodeId = u32;

#[derive(Debug, Clone, PartialEq)]
pub struct AstModule {
    pub module_decl: Option<ModuleDecl>,
    pub imports: Vec<ImportDecl>,
    pub declarations: Vec<Decl>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDecl {
    pub name: ModuleName,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleName {
    pub segments: Vec<String>,
}

impl ModuleName {
    pub fn as_string(&self) -> String {
        self.segments.join(".")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub kind: ImportKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    Module {
        module: ModuleName,
        alias: Option<String>,
    },
    From {
        module: ModuleName,
        names: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Const(ConstDecl),
    Record(RecordDecl),
    Enum(EnumDecl),
    Extern(ExternDecl),
    Action(ActionDecl),
    Test(TestDecl),
}

impl Decl {
    pub fn name(&self) -> &str {
        match self {
            Decl::Const(decl) => &decl.name,
            Decl::Record(decl) => &decl.name,
            Decl::Enum(decl) => &decl.name,
            Decl::Extern(decl) => &decl.name,
            Decl::Action(decl) => &decl.name,
            Decl::Test(decl) => &decl.name,
        }
    }

    pub fn span(&self) -> &SourceSpan {
        match self {
            Decl::Const(decl) => &decl.span,
            Decl::Record(decl) => &decl.span,
            Decl::Enum(decl) => &decl.span,
            Decl::Extern(decl) => &decl.span,
            Decl::Action(decl) => &decl.span,
            Decl::Test(decl) => &decl.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub name: String,
    pub ty: TypeRef,
    pub value: Expr,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordDecl {
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: TypeRef,
    pub meaning: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<VariantField>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariantField {
    pub name: String,
    pub ty: TypeRef,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternDecl {
    pub name: String,
    pub purity: Option<Purity>,
    pub params: Vec<Param>,
    pub return_type: TypeRef,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purity {
    Pure,
    Impure,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub body: Vec<Stmt>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: TypeRef,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestDecl {
    pub name: String,
    pub body: Vec<Stmt>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub id: NodeId,
    pub kind: StmtKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    IntentBlock {
        goal: Option<String>,
        constraints: Vec<String>,
        assumptions: Vec<String>,
        properties: Vec<String>,
    },
    ExplainBlock {
        lines: Vec<String>,
    },
    StepBlock {
        label: String,
        body: Vec<Stmt>,
    },
    RequiresClause {
        condition: Expr,
    },
    EnsuresClause {
        condition: Expr,
    },
    ExampleBlock {
        name: String,
        inputs: Vec<(String, Expr)>,
        outputs: Vec<(String, Expr)>,
    },
    Let {
        name: String,
        explicit_type: Option<TypeRef>,
        value: Expr,
    },
    Var {
        name: String,
        explicit_type: Option<TypeRef>,
        value: Expr,
    },
    Assign {
        target: Target,
        value: Expr,
    },
    If {
        branches: Vec<ConditionalBranch>,
        else_branch: Vec<Stmt>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    ForEach {
        binding: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    Expect(Expr),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalBranch {
    pub condition: Expr,
    pub body: Vec<Stmt>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub id: NodeId,
    pub kind: ExprKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    Dec(String),
    String(String),
    Bool(bool),
    None,
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Tuple(Vec<Expr>),
    Name(String),
    Call {
        callee: Box<Expr>,
        args: Vec<CallArg>,
    },
    FieldAccess {
        base: Box<Expr>,
        field: String,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallArg {
    pub name: Option<String>,
    pub expr: Expr,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Name {
        name: String,
        span: SourceSpan,
    },
    Field {
        base: Box<Target>,
        field: String,
        span: SourceSpan,
    },
    Index {
        base: Box<Target>,
        index: Expr,
        span: SourceSpan,
    },
}

impl Target {
    pub fn span(&self) -> &SourceSpan {
        match self {
            Target::Name { span, .. } => span,
            Target::Field { span, .. } => span,
            Target::Index { span, .. } => span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRef {
    Named(String),
    Generic {
        name: String,
        args: Vec<TypeRef>,
    },
    Tuple(Vec<TypeRef>),
    Action {
        params: Vec<TypeRef>,
        result: Box<TypeRef>,
    },
}
