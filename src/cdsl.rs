use std::rc::Rc;

pub enum CExpr {
    BinOp {
        op: String,
        left: Rc<CExpr>,
        right: Rc<CExpr>,
    },
    PreUnOp {
        op: String,
        ex: Rc<CExpr>,
    },
    PostUnOp {
        op: String,
        ex: Rc<CExpr>,
    },
    ArrIndexOp {
        index: Rc<CExpr>,
        expr: Rc<CExpr>,
    },
    FunCallOp {
        expr: Rc<CExpr>,
        ands: Vec<Rc<CExpr>>,
    },
    Cast {
        ex: Rc<CExpr>,
        typ: Rc<CType>,
    },
    Lit(String),
}

pub enum CType {
    Ptr { to: Rc<CType> },
    Arr { of: Rc<CType>, len: usize },
    Int { size: usize, sign: bool },
    Struct { name: String },
    Union { name: String },
}

pub enum CStmt {
    IF {
        cond: Rc<CExpr>,
        body: Rc<CStmt>,
    },
    While {
        cond: Rc<CExpr>,
        body: Rc<CStmt>,
    },
    For {
        init: Rc<CExpr>,
        test: Rc<CExpr>,
        updt: Rc<CExpr>,
        body: Rc<CStmt>,
    },
    Block(Vec<Rc<CStmt>>),
    Expr(Rc<CExpr>),
}

pub enum CDecl {
    Fun {
        name: String,
        typ: Rc<CType>,
        args: Vec<(String, Rc<CType>)>,
        body: Vec<Rc<CStmt>>,
    },
    Struct {
        name: String,
        members: Vec<(String, Rc<CType>)>,
    },
    Union {
        name: String,
        members: Vec<(String, Rc<CType>)>,
    },
    Var {
        name: String,
        typ: Rc<CType>,
        init: Option<Rc<CExpr>>,
    },
}

pub trait ToC {
    fn export_internal(&self, s: &mut String);
    fn export(&self) -> String {
        let mut s = String::new();
        self.export_internal(&mut s);
        s
    }
}

macro_rules! export_helper {
    ($s:ident, str $e:expr) => ( $s.push_str($e) );
    ($s:ident, chr $e:expr) => ( $s.push($e) );
    ($s:ident, exp $e:ident) => ( $e.export_internal(&mut $s) );
    ($s:ident, vec $e:expr) => ( for elem in $e { elem.export_internal(&mut $s); } );
    ($s:ident, $cmd:tt $op:tt $(, $innercmd:tt $innerop:tt)*) => {{
        export_helper!($s, $cmd $op);
        export_helper!($s, $($innercmd $innerop),*);
    }};
}

impl ToC for CExpr {
    fn export_internal(&self, mut s: &mut String) {
        use self::CExpr::*;

        match self {
            BinOp { op, left, right } => {
                export_helper!(s, chr '(', exp left, chr ')', str op, chr '(', exp right, chr ')')
            }
            PreUnOp { op, ex } => export_helper!(s, chr '(', exp ex, chr ')', str op),
            PostUnOp { op, ex } => export_helper!(s, str op, chr '(', exp ex, chr ')'),
            ArrIndexOp { index, expr } => {
                export_helper!(s, chr '(', exp expr, str ")[", exp index, chr ')')
            }
            FunCallOp { expr, ands } => {
                export_helper!(s, chr '(', exp expr, str ")(");

                let mut it = ands.iter();

                if let Some(expr) = it.next() {
                    expr.export_internal(&mut s);
                }

                for expr in it {
                    s.push(',');
                    expr.export_internal(&mut s);
                }

                export_helper!(s, chr ')');
            }
            Cast { ex, typ } => export_helper!(s, chr '(', exp typ , str ")(", exp ex, chr ')'),
            Lit(lit) => export_helper!(s, chr '(', str lit, chr ')'),
        }
    }
}

impl ToC for CType {
    fn export_internal(&self, s: &mut String) {
        s.push_str(&self.export_with_name(""));
    }
}

impl CType {
    fn export_with_name(&self, name: &str) -> String {
        use self::CType::*;

        let mut typ = Some(self);
        let mut gen = name.to_owned();

        while let Some(typ_o) = typ.take() {
            gen = match typ_o {
                Ptr {..} => format!("*{}", gen),
                Arr {of: _, len} => format!("({})[{}]", gen, len),
                Int {size, sign} => {
                    let name = format!("{}int{}_t",
                                       if *sign { "u" } else { "" },
                                       size);
                    format!("{} {}", name, gen)
                },
                Struct {name} => format!("struct {} {}", name, gen),
                Union {name} => format!("union {} {}", name, gen),
            };

            match typ_o {
                Ptr {to} => typ = Some(to),
                Arr {of, ..} => typ = Some(of),
                _ => (),
            };
        }

        return gen;
    }
}

impl ToC for CStmt {
    fn export_internal(&self, mut s: &mut String) {
        use self::CStmt::*;

        match self {
            IF {cond, body} => export_helper!(s, str "if (", exp cond, chr ')', exp body),
            While {cond, body} => export_helper!(s, str "while (", exp cond, chr ')', exp body),
            For {init, test, updt, body} => export_helper!(s, str "for (", exp init, chr ';', exp test, chr ';', exp updt, chr ')', exp body),
            Block(body) => export_helper!(s, chr '{', vec body, chr '}'),
            Expr(body) => export_helper!(s, exp body, chr ';'),
        }
    }
}

impl ToC for CDecl {
    fn export_internal(&self, mut s: &mut String) {
        use self::CDecl::*;

        match self {
            Fun {name, typ, args, body} => {
                let mut f = String::new();

                f.push_str(&name);
                f.push('(');

                let mut it = args.iter();

                if let Some((aname, atyp)) = it.next() {
                    f.push_str(&atyp.export_with_name(aname));
                }

                for (aname, atyp) in it {
                    s.push(',');
                    f.push_str(&atyp.export_with_name(aname));
                }

                f.push(')');

                s.push_str(&typ.export_with_name(&f));

                export_helper!(s, chr '{', vec body, chr '}');
            },
            Struct {name, members} => {
                export_helper!(s, str "struct ", str name, chr '{');

                for (aname, atyp) in members {
                    s.push_str(&atyp.export_with_name(aname));
                    s.push(';');
                }
                s.push(';');
            },
            Union {name, members} => {
                export_helper!(s, str "union ", str name, chr '{');

                for (aname, atyp) in members {
                    s.push_str(&atyp.export_with_name(aname));
                    s.push(';');
                }
                s.push(';');
            },
            Var {name, typ, init} => {
                s.push_str(&typ.export_with_name(name));
                if let Some(init) = init {
                    export_helper!(s, exp init);
                }
                s.push(';');
            }

        }
    }
}
