use im::HashMap;
use sexp::Atom::*;
use sexp::*;
use std::env;
use std::fs::File;
use std::io::prelude::*;

// ============= Tagged Value Constants =============
const TRUE_VAL: i64 = 3;   // 0b11
const FALSE_VAL: i64 = 1;  // 0b01

// ============= Abstract Syntax Tree =============
#[derive(Debug)]
enum Op1 {
    Add1,
    Sub1,
    Negate,
    IsNum,
    IsBool,
}

#[derive(Debug)]
enum Op2 {
    Plus,
    Minus,
    Times,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    Equal,
}

#[derive(Debug)]
enum Expr {
    Number(i32),
    Boolean(bool),
    Input,
    Id(String),
    Let(Vec<(String, Expr)>, Box<Expr>),
    UnOp(Op1, Box<Expr>),
    BinOp(Op2, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Block(Vec<Expr>),
    Loop(Box<Expr>),
    Break(Box<Expr>),
    Set(String, Box<Expr>),
}

// ============= Parsing =============
fn is_reserved(name: &str) -> bool {
    matches!(
        name,
        "let" | "add1" | "sub1" | "negate" | "isnum" | "isbool"
            | "if" | "block" | "loop" | "break" | "set!"
            | "true" | "false" | "input"
            | "+" | "-" | "*" | "<" | ">" | "<=" | ">=" | "="
    )
}

fn parse_expr(s: &Sexp) -> Expr {
    match s {
        Sexp::Atom(I(n)) => {
            let val = i32::try_from(*n).unwrap_or_else(|_| panic!("Invalid: number out of range"));
            Expr::Number(val)
        }
        Sexp::Atom(S(name)) => match name.as_str() {
            "true"  => Expr::Boolean(true),
            "false" => Expr::Boolean(false),
            "input" => Expr::Input,
            _ => {
                if is_reserved(name) {
                    panic!("Invalid: reserved word used as identifier: {}", name);
                }
                Expr::Id(name.clone())
            }
        },
        Sexp::List(vec) => match &vec[..] {
            // Unary ops
            [Sexp::Atom(S(op)), e] if op == "add1"   => Expr::UnOp(Op1::Add1,   Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "sub1"   => Expr::UnOp(Op1::Sub1,   Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "negate" => Expr::UnOp(Op1::Negate, Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "isnum"  => Expr::UnOp(Op1::IsNum,  Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "isbool" => Expr::UnOp(Op1::IsBool, Box::new(parse_expr(e))),

            // Loop and break
            [Sexp::Atom(S(op)), e] if op == "loop"  => Expr::Loop(Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "break" => Expr::Break(Box::new(parse_expr(e))),

            // Set!
            [Sexp::Atom(S(op)), Sexp::Atom(S(name)), e] if op == "set!" => {
                if is_reserved(name) {
                    panic!("Invalid: cannot set! reserved word: {}", name);
                }
                Expr::Set(name.clone(), Box::new(parse_expr(e)))
            }

            // Binary ops
            [Sexp::Atom(S(op)), e1, e2] if op == "+"  => Expr::BinOp(Op2::Plus,     Box::new(parse_expr(e1)), Box::new(parse_expr(e2))),
            [Sexp::Atom(S(op)), e1, e2] if op == "-"  => Expr::BinOp(Op2::Minus,    Box::new(parse_expr(e1)), Box::new(parse_expr(e2))),
            [Sexp::Atom(S(op)), e1, e2] if op == "*"  => Expr::BinOp(Op2::Times,    Box::new(parse_expr(e1)), Box::new(parse_expr(e2))),
            [Sexp::Atom(S(op)), e1, e2] if op == "<"  => Expr::BinOp(Op2::Less,     Box::new(parse_expr(e1)), Box::new(parse_expr(e2))),
            [Sexp::Atom(S(op)), e1, e2] if op == ">"  => Expr::BinOp(Op2::Greater,  Box::new(parse_expr(e1)), Box::new(parse_expr(e2))),
            [Sexp::Atom(S(op)), e1, e2] if op == "<=" => Expr::BinOp(Op2::LessEq,   Box::new(parse_expr(e1)), Box::new(parse_expr(e2))),
            [Sexp::Atom(S(op)), e1, e2] if op == ">=" => Expr::BinOp(Op2::GreaterEq,Box::new(parse_expr(e1)), Box::new(parse_expr(e2))),
            [Sexp::Atom(S(op)), e1, e2] if op == "="  => Expr::BinOp(Op2::Equal,    Box::new(parse_expr(e1)), Box::new(parse_expr(e2))),

            // If
            [Sexp::Atom(S(op)), cond, then_e, else_e] if op == "if" => {
                Expr::If(
                    Box::new(parse_expr(cond)),
                    Box::new(parse_expr(then_e)),
                    Box::new(parse_expr(else_e)),
                )
            }

            // Block
            [Sexp::Atom(S(op)), rest @ ..] if op == "block" => {
                if rest.is_empty() {
                    panic!("Invalid: block requires at least one expression");
                }
                Expr::Block(rest.iter().map(parse_expr).collect())
            }

            // Let (fixed to handle multiple body expressions)
            [Sexp::Atom(S(op)), Sexp::List(bindings), body @ ..] if op == "let" => {
                if bindings.is_empty() {
                    panic!("Invalid: let requires at least one binding");
                }
                let mut seen = std::collections::HashSet::new();
                let parsed_binds: Vec<(String, Expr)> = bindings.iter().map(|b| {
                    let (name, expr) = parse_bind(b);
                    if !seen.insert(name.clone()) {
                        panic!("Duplicate binding: {}", name);
                    }
                    (name, expr)
                }).collect();

                let body_expr = if body.len() == 1 {
                    parse_expr(&body[0])
                } else {
                    Expr::Block(body.iter().map(parse_expr).collect())
                };

                Expr::Let(parsed_binds, Box::new(body_expr))
            }

            _ => panic!("Invalid: unrecognized expression: {:?}", s),
        },
        _ => panic!("Invalid: unexpected sexp: {:?}", s),
    }
}

fn parse_bind(s: &Sexp) -> (String, Expr) {
    match s {
        Sexp::List(vec) => match &vec[..] {
            [Sexp::Atom(S(name)), e] => {
                if is_reserved(name) {
                    panic!("Invalid: reserved word used as binding name: {}", name);
                }
                (name.clone(), parse_expr(e))
            }
            _ => panic!("Invalid: bad binding form"),
        },
        _ => panic!("Invalid: binding must be a list"),
    }
}

// ============= Compilation =============
// si: next available stack slot index (slot n => offset -8*n from RSP)
// env: maps variable name -> stack offset (negative number)
// label_counter: global counter for generating unique labels
// break_target: label to jump to when break is encountered

fn new_label(label_counter: &mut i32, name: &str) -> String {
    *label_counter += 1;
    format!("{}_{}", name, label_counter)
}

/// Emit runtime type-check: error code 1 ("invalid argument") if RAX is not a number.
/// A number has LSB == 0.
fn check_is_num(label_counter: &mut i32) -> String {
    let ok_label = new_label(label_counter, "num_check_ok");
    format!(
        "mov rbx, rax
  test rbx, 1
  je {ok}
  mov rdi, 1
  call snek_error
{ok}:",
        ok = ok_label
    )
}

/// Emit runtime check that both rax and the value at [rsp + offset] are numbers.
fn check_both_num(offset: i32, label_counter: &mut i32) -> String {
    let ok_label = new_label(label_counter, "both_num_ok");
    // Check right (RAX)
    // Check left ([rsp+offset])
    // OR the tags together; if any bit is 1, not both numbers
    format!(
        "mov rbx, rax
  or rbx, qword {mem}
  test rbx, 1
  je {ok}
  mov rdi, 1
  call snek_error
{ok}:",
        mem = offset_str(offset),
        ok = ok_label
    )
}

fn offset_str(offset: i32) -> String {
    if offset < 0 {
        format!("[rsp - {}]", -offset)
    } else {
        format!("[rsp + {}]", offset)
    }
}

fn compile_expr(
    e: &Expr,
    si: i32,
    env: &HashMap<String, i32>,
    label_counter: &mut i32,
    break_target: &Option<String>,
) -> String {
    match e {
        // ---- Literals ----
        Expr::Number(n) => {
            // Encode: shift left by 1
            let tagged = (*n as i64) << 1;
            format!("mov rax, {}", tagged)
        }

        Expr::Boolean(b) => {
            let val = if *b { TRUE_VAL } else { FALSE_VAL };
            format!("mov rax, {}", val)
        }

        Expr::Input => {
            // input is passed in RDI by the runtime
            "mov rax, rdi".to_string()
        }

        // ---- Variable ----
        Expr::Id(name) => {
            let offset = env
                .get(name)
                .unwrap_or_else(|| panic!("Unbound variable identifier {}", name));
            format!("mov rax, qword {}", offset_str(*offset))
        }

        // ---- Unary Ops ----
        Expr::UnOp(op, expr) => {
            let mut parts = vec![compile_expr(expr, si, env, label_counter, break_target)];
            match op {
                Op1::Add1 => {
                    // add1 on tagged number: add 2 (since numbers are shifted left by 1)
                    parts.push(check_is_num(label_counter));
                    parts.push("add rax, 2".to_string());
                    // Overflow check
                    let ok = new_label(label_counter, "ovf_ok");
                    parts.push(format!("jno {ok}\n  mov rdi, 2\n  call snek_error\n{ok}:", ok = ok));
                }
                Op1::Sub1 => {
                    parts.push(check_is_num(label_counter));
                    parts.push("sub rax, 2".to_string());
                    let ok = new_label(label_counter, "ovf_ok");
                    parts.push(format!("jno {ok}\n  mov rdi, 2\n  call snek_error\n{ok}:", ok = ok));
                }
                Op1::Negate => {
                    parts.push(check_is_num(label_counter));
                    parts.push("neg rax".to_string());
                    let ok = new_label(label_counter, "ovf_ok");
                    parts.push(format!("jno {ok}\n  mov rdi, 2\n  call snek_error\n{ok}:", ok = ok));
                }
                Op1::IsNum => {
                    // Result: true if LSB == 0, false otherwise
                    parts.push(format!(
                        "and rax, 1
  cmp rax, 0
  mov rax, {true_val}
  mov rbx, {false_val}
  cmovne rax, rbx",
                        true_val = TRUE_VAL,
                        false_val = FALSE_VAL
                    ));
                }
                Op1::IsBool => {
                    // Result: true if LSB == 1, false otherwise
                    parts.push(format!(
                        "and rax, 1
  cmp rax, 1
  mov rax, {true_val}
  mov rbx, {false_val}
  cmovne rax, rbx",
                        true_val = TRUE_VAL,
                        false_val = FALSE_VAL
                    ));
                }
            }
            parts.join("\n  ")
        }

        // ---- Binary Ops ----
        Expr::BinOp(op, left, right) => {
            let mut parts = vec![];
            let offset = -8 * si;

            // Compile left → RAX, spill to stack
            parts.push(compile_expr(left, si, env, label_counter, break_target));
            parts.push(format!("mov qword {}, rax", offset_str(offset)));

            // Compile right → RAX (use si+1 to avoid clobber)
            parts.push(compile_expr(right, si + 1, env, label_counter, break_target));

            match op {
                Op2::Plus => {
                    parts.push(check_both_num(offset, label_counter));
                    parts.push(format!("add rax, qword {}", offset_str(offset)));
                    let ok = new_label(label_counter, "ovf_ok");
                    parts.push(format!("jno {ok}\n  mov rdi, 2\n  call snek_error\n{ok}:", ok = ok));
                }
                Op2::Minus => {
                    parts.push(check_both_num(offset, label_counter));
                    // left - right: load left into rbx, subtract rax
                    parts.push(format!(
                        "mov rbx, qword {mem}
  sub rbx, rax
  mov rax, rbx",
                        mem = offset_str(offset)
                    ));
                    let ok = new_label(label_counter, "ovf_ok");
                    parts.push(format!("jno {ok}\n  mov rdi, 2\n  call snek_error\n{ok}:", ok = ok));
                }
                Op2::Times => {
                    parts.push(check_both_num(offset, label_counter));
                    // Both tagged: (2a) * (2b) = 4ab, but we want 2(a*b).
                    // So shift right (untag) one operand first, then imul.
                    parts.push(format!(
                        "sar rax, 1
  imul rax, qword {}",
                        offset_str(offset)
                    ));
                    let ok = new_label(label_counter, "ovf_ok");
                    parts.push(format!("jno {ok}\n  mov rdi, 2\n  call snek_error\n{ok}:", ok = ok));
                }
                Op2::Less | Op2::Greater | Op2::LessEq | Op2::GreaterEq => {
                    parts.push(check_both_num(offset, label_counter));
                    // Compare left vs right
                    parts.push(format!(
                        "mov rbx, qword {mem}
  cmp rbx, rax",
                        mem = offset_str(offset)
                    ));
                    let set_instr = match op {
                        Op2::Less     => "setl",
                        Op2::Greater  => "setg",
                        Op2::LessEq   => "setle",
                        Op2::GreaterEq=> "setge",
                        _ => unreachable!(),
                    };
                    // Convert 0/1 in AL to false/true tagged
                    parts.push(format!(
                        "{set} al
  movzx rax, al
  ; rax is 0 or 1; convert to tagged bool
  ; true=3 (0b11), false=1 (0b01)
  ; formula: (rax * 2) + 1
  imul rax, 2
  add rax, 1",
                        set = set_instr
                    ));
                }
                Op2::Equal => {
                    // Equal requires same type; error if types differ
                    // Check: (left_tag XOR right_tag) & 1 != 0 => type mismatch
                    let ok_label = new_label(label_counter, "eq_type_ok");
                    parts.push(format!(
                        "mov rbx, qword {mem}
  mov rcx, rax
  xor rcx, rbx
  test rcx, 1
  je {ok}
  mov rdi, 1
  call snek_error
{ok}:
  cmp rbx, rax
  mov rax, {true_val}
  mov rbx, {false_val}
  cmovne rax, rbx",
                        mem = offset_str(offset),
                        ok = ok_label,
                        true_val = TRUE_VAL,
                        false_val = FALSE_VAL,
                    ));
                }
            }
            parts.join("\n  ")
        }

        // ---- If ----
        Expr::If(cond, then_e, else_e) => {
            let else_label = new_label(label_counter, "if_else");
            let end_label  = new_label(label_counter, "if_end");

            let cond_instrs = compile_expr(cond, si, env, label_counter, break_target);
            let then_instrs = compile_expr(then_e, si, env, label_counter, break_target);
            let else_instrs = compile_expr(else_e, si, env, label_counter, break_target);

            format!(
                "{cond}
  cmp rax, {false_val}
  je {else_l}
  {then}
  jmp {end_l}
{else_l}:
  {else_}
{end_l}:",
                cond      = cond_instrs,
                false_val = FALSE_VAL,
                else_l    = else_label,
                then      = then_instrs,
                end_l     = end_label,
                else_     = else_instrs,
            )
        }

        // ---- Block ----
        Expr::Block(exprs) => {
            exprs
                .iter()
                .map(|e| compile_expr(e, si, env, label_counter, break_target))
                .collect::<Vec<_>>()
                .join("\n  ")
        }

        // ---- Loop ----
        Expr::Loop(body) => {
            let loop_start = new_label(label_counter, "loop_start");
            let loop_end   = new_label(label_counter, "loop_end");

            let body_instrs = compile_expr(body, si, env, label_counter, &Some(loop_end.clone()));

            format!(
                "{start}:
  {body}
  jmp {start}
{end}:",
                start = loop_start,
                body  = body_instrs,
                end   = loop_end,
            )
        }

        // ---- Break ----
        Expr::Break(expr) => {
            match break_target {
                None => panic!("Invalid: break used outside of loop"),
                Some(label) => {
                    let val_instrs = compile_expr(expr, si, env, label_counter, break_target);
                    format!(
                        "{val}
  jmp {label}",
                        val   = val_instrs,
                        label = label
                    )
                }
            }
        }

        // ---- Set! ----
        Expr::Set(name, expr) => {
            let offset = env
                .get(name)
                .unwrap_or_else(|| panic!("Unbound variable identifier {}", name));
            let val_instrs = compile_expr(expr, si, env, label_counter, break_target);
            format!(
                "{val}
  mov qword {mem}, rax",
                val = val_instrs,
                mem = offset_str(*offset),
            )
        }

        // ---- Let ----
        Expr::Let(bindings, body) => {
            let mut parts = vec![];
            let mut new_env = env.clone();
            let mut cur_si = si;

            for (name, bind_expr) in bindings {
                parts.push(compile_expr(bind_expr, cur_si, &new_env, label_counter, break_target));
                let offset = -8 * cur_si;
                parts.push(format!("mov qword {}, rax", offset_str(offset)));
                new_env = new_env.update(name.clone(), offset);
                cur_si += 1;
            }

            parts.push(compile_expr(body, cur_si, &new_env, label_counter, break_target));
            parts.join("\n  ")
        }
    }
}

// ============= Stack Size Computation =============
fn compute_max_si(e: &Expr, si: i32) -> i32 {
    match e {
        Expr::Number(_) | Expr::Boolean(_) | Expr::Input | Expr::Id(_) => si,
        Expr::UnOp(_, sub) => compute_max_si(sub, si),
        Expr::BinOp(_, left, right) => {
            let after_left = compute_max_si(left, si);
            let right_si   = std::cmp::max(after_left, si + 1);
            compute_max_si(right, right_si)
        }
        Expr::If(cond, then_e, else_e) => {
            let a = compute_max_si(cond, si);
            let b = compute_max_si(then_e, si);
            let c = compute_max_si(else_e, si);
            std::cmp::max(a, std::cmp::max(b, c))
        }
        Expr::Block(exprs) => exprs.iter().fold(si, |acc, e| std::cmp::max(acc, compute_max_si(e, si))),
        Expr::Loop(body)   => compute_max_si(body, si),
        Expr::Break(expr)  => compute_max_si(expr, si),
        Expr::Set(_, expr) => compute_max_si(expr, si),
        Expr::Let(bindings, body) => {
            let mut cur_si = si;
            for (_, bind_expr) in bindings {
                let after = compute_max_si(bind_expr, cur_si);
                cur_si = std::cmp::max(after, cur_si) + 1;
            }
            compute_max_si(body, cur_si)
        }
    }
}

// ============= Top-Level Compile =============
fn compile(e: &Expr) -> String {
    let env: HashMap<String, i32> = HashMap::new();
    let start_si = 2; // slot 1 reserved; first usable is slot 2
    let mut label_counter = 0i32;

    let max_si = compute_max_si(e, start_si);
    let num_slots = std::cmp::max(max_si - start_si, 0);
    let bytes_raw = num_slots * 8;
    // Align to 16 bytes
    let bytes_needed = if bytes_raw == 0 { 0 } else { (bytes_raw + 15) & !15 };

    let mut parts: Vec<String> = vec![];

    if bytes_needed > 0 {
        parts.push(format!("sub rsp, {}", bytes_needed));
    }

    parts.push(compile_expr(e, start_si, &env, &mut label_counter, &None));

    if bytes_needed > 0 {
        parts.push(format!("add rsp, {}", bytes_needed));
    }

    parts.join("\n  ")
}

// ============= Main =============
fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input.snek> <output.s>", args[0]);
        std::process::exit(1);
    }
    let in_name  = &args[1];
    let out_name = &args[2];

    let mut in_file = File::open(in_name)?;
    let mut in_contents = String::new();
    in_file.read_to_string(&mut in_contents)?;

    let sexp = parse(&in_contents).unwrap_or_else(|_| panic!("Invalid"));
    let expr = parse_expr(&sexp);
    let instrs = compile(&expr);

    // The assembly uses snek_error for runtime errors (defined in start.rs / runtime)
    let asm_program = format!(
        "section .text
extern snek_error
global our_code_starts_here
our_code_starts_here:
  {instrs}
  ret
",
        instrs = instrs
    );

    let mut out_file = File::create(out_name)?;
    out_file.write_all(asm_program.as_bytes())?;
    Ok(())
}