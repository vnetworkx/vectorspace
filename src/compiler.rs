use crate::ast::*;
use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::kernel::KernelOp;
use crate::value::{Context, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerConfig {
    pub certification_threshold: String,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self { certification_threshold: "0.8000".to_string() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilationUnit {
    pub ops: Vec<KernelOp>,
}

pub fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Integer(v) => v.to_string(),
        Expr::String(v) => v.clone(),
        Expr::Ident(v) => v.clone(),
        Expr::Path(parts) => parts.join("."),
        Expr::Call { callee, args } => {
            let mut out = String::new();
            out.push_str(callee);
            out.push('(');
            for (idx, arg) in args.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                if let Some(name) = &arg.name {
                    out.push_str(name);
                    out.push('=');
                    out.push_str(&expr_to_string(&arg.value));
                } else {
                    out.push_str(&expr_to_string(&arg.value));
                }
            }
            out.push(')');
            out
        }
        Expr::Group(inner) => expr_to_string(inner),
    }
}

pub fn eval_int(expr: &Expr, env: &BTreeMap<String, u128>) -> Result<u128, RuntimeError> {
    match expr {
        Expr::Integer(v) => Ok(*v),
        Expr::Ident(name) => env.get(name).copied().ok_or_else(|| RuntimeError::new(RuntimeErrorKind::UnknownSymbol(name.clone()))),
        Expr::Group(inner) => eval_int(inner, env),
        other => Err(RuntimeError::new(RuntimeErrorKind::InvalidOperation(format!(
            "expected integer expression, found {other:?}"
        )))),
    }
}

pub fn eval_context(expr: &ContextExpr) -> Context {
    let mut context = Context::new();
    for arg in &expr.args {
        if let Some(name) = &arg.name {
            context.fields.insert(name.clone(), value_from_expr(&arg.value));
        }
    }
    context
}

pub fn value_from_expr(expr: &Expr) -> Value {
    match expr {
        Expr::Integer(v) => Value::Int(*v),
        Expr::String(v) => Value::String(v.clone()),
        Expr::Ident(v) => Value::String(v.clone()),
        Expr::Path(parts) => Value::Path(parts.clone()),
        Expr::Call { callee, args } => {
            let mut map = BTreeMap::new();
            for arg in args {
                if let Some(name) = &arg.name {
                    map.insert(name.clone(), value_from_expr(&arg.value));
                }
            }
            let mut out = BTreeMap::new();
            out.insert("callee".to_string(), Value::String(callee.clone()));
            out.insert("args".to_string(), Value::Map(map));
            Value::Map(out)
        }
        Expr::Group(inner) => value_from_expr(inner),
    }
}

pub fn compile_program(program: &Program) -> Result<CompilationUnit, RuntimeError> {
    let mut ops = Vec::new();
    let mut projection_seq: u64 = 0;
    for statement in &program.statements {
        match statement {
            Statement::VectorDecl(v) => ops.push(KernelOp::CreateVector {
                name: v.name.clone(),
                vector_type: v.vector_type,
                components: v.components.clone(),
            }),
            Statement::WalletDecl(w) => ops.push(KernelOp::BindWallet {
                name: w.name.clone(),
                public_key: w.public_key.clone(),
            }),
            Statement::Certify(c) => {
                let ctx = eval_context(&c.context);
                let mut context_text = String::new();
                context_text.push_str("ctx(");
                for (idx, (k, v)) in ctx.fields.iter().enumerate() {
                    if idx > 0 {
                        context_text.push_str(", ");
                    }
                    context_text.push_str(k);
                    context_text.push('=');
                    context_text.push_str(&format!("{v:?}"));
                }
                context_text.push(')');

                let ratio = compute_auth_ratio(&ctx);
                let certified = ratio.parse::<f64>().unwrap_or(0.0) >= 0.8000;
                ops.push(KernelOp::Certify {
                    target: c.target.clone(),
                    context: context_text,
                    auth_ratio: ratio,
                    certified,
                });
            }
            Statement::Transfer(t) => {
                let mut env = BTreeMap::new();
                let amount = eval_int(&t.amount, &env)?;
                let drain = t
                    .drain
                    .as_ref()
                    .map(|expr| eval_int(expr, &env))
                    .transpose()?
                    .unwrap_or(0);
                let policy = t.policy.as_ref().map(expr_to_string);
                ops.push(KernelOp::Transfer {
                    source: t.source.clone(),
                    destination: t.destination.clone(),
                    amount,
                    drain,
                    policy,
                });
            }
            Statement::Drain(d) => {
                let env = BTreeMap::new();
                let amount = eval_int(&d.amount, &env)?;
                ops.push(KernelOp::Drain { target: d.target.clone(), amount });
            }
            Statement::Project(p) => {
                let env = BTreeMap::new();
                let amount = eval_int(&p.amount, &env)?;
                let policy = p.policy.as_ref().map(expr_to_string);
                projection_seq = projection_seq.saturating_add(1);
                let projection_id = format!("proj_{:03}", projection_seq);
                ops.push(KernelOp::Project {
                    source: p.source.clone(),
                    environment: p.environment.clone(),
                    amount,
                    policy,
                    projection_id,
                });
            }
            Statement::Reconstruct(r) => {
                ops.push(KernelOp::Reconstruct {
                    target: r.target.clone(),
                    projection_id: r.projection_id.clone(),
                });
            }
            Statement::Query(q) => {
                ops.push(KernelOp::Query { expr: expr_to_string(&q.expr) });
            }
            Statement::Record(r) => {
                ops.push(KernelOp::Record { expr: expr_to_string(&r.expr) });
            }
            Statement::Contract(c) => {
                let actions = c.actions.iter().map(|a| {
                    let mut text = a.name.clone();
                    text.push('(');
                    for (idx, arg) in a.args.iter().enumerate() {
                        if idx > 0 {
                            text.push_str(", ");
                        }
                        if let Some(name) = &arg.name {
                            text.push_str(name);
                            text.push('=');
                            text.push_str(&expr_to_string(&arg.value));
                        } else {
                            text.push_str(&expr_to_string(&arg.value));
                        }
                    }
                    text.push(')');
                    text
                }).collect();
                ops.push(KernelOp::Contract { name: c.name.clone(), actions });
                for action in &c.actions {
                    ops.push(KernelOp::ContractAction {
                        contract: c.name.clone(),
                        action: action.name.clone(),
                        args: action.args.iter().map(|a| match &a.name {
                            Some(name) => format!("{name}={}", expr_to_string(&a.value)),
                            None => expr_to_string(&a.value),
                        }).collect(),
                    });
                }
            }
        }
    }
    Ok(CompilationUnit { ops })
}

fn compute_auth_ratio(ctx: &Context) -> String {
    let mut am = 1.0f64;
    let mut ac = 1.0f64;
    let mut ao = 1.0f64;
    let mut ap = 1.0f64;

    if let Some(Value::String(op)) = ctx.get("op") {
        if op.trim().is_empty() {
            ap = 0.0;
        }
    } else {
        ap = 0.75;
    }

    if let Some(Value::String(space)) = ctx.get("space") {
        if space.trim().is_empty() {
            ac = 0.5;
        }
    }

    if let Some(Value::String(risk)) = ctx.get("risk") {
        if risk == "high" {
            ao = 0.8;
        }
    }

    if ctx.fields.is_empty() {
        am = 0.5;
        ac = 0.5;
        ao = 0.5;
        ap = 0.5;
    }

    let ratio = 0.35 * am + 0.25 * ac + 0.25 * ao + 0.15 * ap;
    format!("{ratio:.4}")
}
