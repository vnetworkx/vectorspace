use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    VectorDecl(VectorDecl),
    WalletDecl(WalletDecl),
    Certify(CertifyStmt),
    Transfer(TransferStmt),
    Drain(DrainStmt),
    Project(ProjectStmt),
    Reconstruct(ReconstructStmt),
    Query(QueryStmt),
    Record(RecordStmt),
    Contract(ContractDecl),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorDecl {
    pub name: String,
    pub vector_type: VectorType,
    pub components: Vec<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletDecl {
    pub name: String,
    pub public_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertifyStmt {
    pub target: String,
    pub context: ContextExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferStmt {
    pub source: String,
    pub destination: String,
    pub amount: Expr,
    pub drain: Option<Expr>,
    pub policy: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrainStmt {
    pub target: String,
    pub amount: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStmt {
    pub source: String,
    pub environment: String,
    pub amount: Expr,
    pub policy: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconstructStmt {
    pub target: String,
    pub projection_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryStmt {
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordStmt {
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractDecl {
    pub name: String,
    pub actions: Vec<ContractAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractAction {
    pub name: String,
    pub args: Vec<NamedArg>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedArg {
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Integer(u128),
    String(String),
    Ident(String),
    Path(Vec<String>),
    Call { callee: String, args: Vec<NamedArg> },
    Group(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextExpr {
    pub args: Vec<NamedArg>,
}

impl ContextExpr {
    pub fn new(args: Vec<NamedArg>) -> Self {
        Self { args }
    }

    pub fn get(&self, key: &str) -> Option<&Expr> {
        self.args.iter().find_map(|arg| match &arg.name {
            Some(name) if name == key => Some(&arg.value),
            _ => None,
        })
    }

    pub fn as_map(&self) -> BTreeMap<String, Expr> {
        let mut out = BTreeMap::new();
        for arg in &self.args {
            if let Some(name) = &arg.name {
                out.insert(name.clone(), arg.value.clone());
            }
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorType {
    Position,
    Free,
    Bound,
    Unit,
    Zero,
    Spatial,
}

impl VectorType {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "position" => Some(Self::Position),
            "free" => Some(Self::Free),
            "bound" => Some(Self::Bound),
            "unit" => Some(Self::Unit),
            "zero" => Some(Self::Zero),
            "spatial" => Some(Self::Spatial),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Position => "position",
            Self::Free => "free",
            Self::Bound => "bound",
            Self::Unit => "unit",
            Self::Zero => "zero",
            Self::Spatial => "spatial",
        }
    }
}
