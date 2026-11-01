//! Top-level declarations and the bodies of `component`, `bus`, `isa` and
//! `system`.

use super::{PResult, Parser};
use crate::ast::*;
use crate::token::{Keyword, Punct, TokenKind};

impl Parser {
    pub(crate) fn parse_decl(&mut self) -> PResult<Decl> {
        let docs = self.take_docs();
        let start = self.cur_span();
        match self.peek_kind().clone() {
            TokenKind::Keyword(Keyword::Use) => {
                self.advance();
                let path = self.parse_path_colons()?;
                let mut items = Vec::new();
                if self.eat_punct(Punct::ColonColon) {
                    items = self.parse_delimited(Punct::LBrace, Punct::RBrace, &[Punct::Comma], |p| p.expect_ident())?;
                }
                self.expect_semi()?;
                Ok(Decl::Use(UseDecl {
                    path,
                    items,
                    span: start.to(self.prev_span()),
                }))
            }
            TokenKind::Keyword(Keyword::Const) => Ok(Decl::Const(self.parse_const(docs)?)),
            TokenKind::Ident(w) => match w.as_str() {
                "enum" => Ok(Decl::Enum(self.parse_enum(docs, "enum")?)),
                "bundle" => Ok(Decl::Bundle(self.parse_bundle(docs)?)),
                "union" => Ok(Decl::Union(self.parse_union(docs)?)),
                "unit" => Ok(Decl::Unit(self.parse_unit(docs)?)),
                "component" => Ok(Decl::Component(self.parse_component(docs)?)),
                "bus" => Ok(Decl::Bus(self.parse_bus(docs)?)),
                "isa" => Ok(Decl::Isa(self.parse_isa(docs)?)),
                "system" => Ok(Decl::System(self.parse_system(docs)?)),
                "test" => {
                    self.advance();
                    let name = self.expect_str()?;
                    let target = if self.eat_word("on") {
                        Some(self.parse_path_dotted()?)
                    } else {
                        None
                    };
                    let body = self.parse_block()?;
                    Ok(Decl::Test(TestDecl {
                        docs,
                        name,
                        target,
                        body,
                        span: start.to(self.prev_span()),
                    }))
                }
                "vectors" => {
                    self.advance();
                    let glob = self.expect_str()?;
                    self.expect_word("on")?;
                    let target = self.parse_path_dotted()?;
                    let mut items = Vec::new();
                    if self.at_punct(Punct::LBrace) {
                        self.advance();
                        items = self.parse_items_until_rbrace(|p| p.parse_vectors_item())?;
                    } else {
                        self.expect_semi()?;
                    }
                    Ok(Decl::Vectors(VectorsDecl {
                        docs,
                        glob,
                        target,
                        items,
                        span: start.to(self.prev_span()),
                    }))
                }
                "conformance" => {
                    self.advance();
                    let target = self.parse_path_dotted()?;
                    let body = self.parse_block()?;
                    Ok(Decl::Conformance(ConformanceDecl {
                        docs,
                        target,
                        body,
                        span: start.to(self.prev_span()),
                    }))
                }
                _ => Err(self.error_expected("a declaration (`enum`, `bundle`, `union`, `const`, `unit`, `component`, `bus`, `isa`, `system`, `test`, `vectors`, `conformance`, `use`)")),
            },
            _ => Err(self.error_expected("a declaration")),
        }
    }

    // -- simple types --------------------------------------------------------

    /// `enum Name : Repr { A, B = 2, }`. `word` is `enum` or `mode`.
    pub(crate) fn parse_enum(&mut self, docs: Docs, word: &str) -> PResult<EnumDecl> {
        let start = self.expect_word(word)?;
        let name = self.expect_ident()?;
        if !self.eat_punct(Punct::Colon) {
            return Err(self.error(
                "E0103",
                self.cur_span(),
                format!(
                    "`{word}` requires an explicit backing width: `{word} {} : u3 {{ ... }}`",
                    name.name
                ),
            ));
        }
        let repr = self.parse_type()?;
        let variants =
            self.parse_delimited(Punct::LBrace, Punct::RBrace, &[Punct::Comma], |p| {
                let docs = p.take_docs();
                let name = p.expect_ident()?;
                let value = if p.eat_punct(Punct::Eq) {
                    Some(p.parse_expr()?)
                } else {
                    None
                };
                let span = name.span.to(p.prev_span());
                Ok(EnumVariant {
                    docs,
                    name,
                    value,
                    span,
                })
            })?;
        Ok(EnumDecl {
            docs,
            name,
            repr,
            variants,
            span: start.to(self.prev_span()),
        })
    }

    pub(crate) fn parse_bundle(&mut self, docs: Docs) -> PResult<BundleDecl> {
        let start = self.expect_word("bundle")?;
        let name = self.expect_ident()?;
        let fields = self.parse_delimited(Punct::LBrace, Punct::RBrace, &[Punct::Comma], |p| {
            let docs = p.take_docs();
            let name = p.expect_ident()?;
            p.expect_punct(Punct::Colon)?;
            let ty = p.parse_type()?;
            let offset = if p.eat_punct(Punct::At) {
                Some(p.parse_expr()?)
            } else {
                None
            };
            let default = if p.eat_punct(Punct::Eq) {
                Some(p.parse_expr()?)
            } else {
                None
            };
            let span = name.span.to(p.prev_span());
            Ok(BundleField {
                docs,
                name,
                ty,
                offset,
                default,
                span,
            })
        })?;
        Ok(BundleDecl {
            docs,
            name,
            fields,
            span: start.to(self.prev_span()),
        })
    }

    fn parse_union(&mut self, docs: Docs) -> PResult<UnionDecl> {
        let start = self.expect_word("union")?;
        let name = self.expect_ident()?;
        let variants =
            self.parse_delimited(Punct::LBrace, Punct::RBrace, &[Punct::Comma], |p| {
                let docs = p.take_docs();
                let name = p.expect_ident()?;
                let payload = if p.at_punct(Punct::LParen) {
                    p.advance();
                    let t = p.parse_type()?;
                    p.expect_punct(Punct::RParen)?;
                    Some(t)
                } else {
                    None
                };
                let span = name.span.to(p.prev_span());
                Ok(UnionVariant {
                    docs,
                    name,
                    payload,
                    span,
                })
            })?;
        Ok(UnionDecl {
            docs,
            name,
            variants,
            span: start.to(self.prev_span()),
        })
    }

    pub(crate) fn parse_const(&mut self, docs: Docs) -> PResult<ConstDecl> {
        let start = self.expect_kw(Keyword::Const)?;
        let name = self.expect_ident()?;
        self.expect_punct(Punct::Colon)?;
        let ty = self.parse_type()?;
        self.expect_punct(Punct::Eq)?;
        let value = self.parse_expr()?;
        self.expect_semi()?;
        Ok(ConstDecl {
            docs,
            name,
            ty,
            value,
            span: start.to(self.prev_span()),
        })
    }

    /// `unit Name<generics>(params) -> Ret { body }`
    pub(crate) fn parse_unit(&mut self, docs: Docs) -> PResult<UnitDecl> {
        let start = self.expect_word("unit")?;
        let name = self.expect_ident()?;
        let generics = self.parse_generic_params()?;
        let params = self.parse_params()?;
        self.expect_punct(Punct::RArrow)?;
        let ret = self.parse_type()?;
        let body = self.parse_block()?;
        Ok(UnitDecl {
            docs,
            name,
            generics,
            params,
            ret,
            body,
            span: start.to(self.prev_span()),
        })
    }

    // -- component -----------------------------------------------------------

    fn parse_component(&mut self, docs: Docs) -> PResult<ComponentDecl> {
        let start = self.expect_word("component")?;
        let name = self.expect_ident()?;
        let generics = self.parse_generic_params()?;
        self.expect_punct(Punct::LBrace)?;
        let mut isa = None;
        let mut ports = Vec::new();
        let mut rtl = None;
        let mut model = None;
        while !self.at_punct(Punct::RBrace) {
            if self.at_eof() {
                return Err(self.error_expected("`}`"));
            }
            let before = self.pos;
            let item_docs = self.take_docs();
            let _ = item_docs;
            let item_start = self.cur_span();
            let result: PResult<()> = (|| {
                if self.eat_word("isa") {
                    let p = self.parse_path_colons()?;
                    self.expect_semi()?;
                    if isa.replace(p).is_some() {
                        self.error("E0104", item_start, "component declares `isa` twice");
                    }
                } else if self.eat_word("ports") {
                    ports.extend(self.parse_ports()?);
                } else if self.at_word("rtl") && self.nth_is_punct(1, Punct::LBrace) {
                    self.advance();
                    let b = self.parse_block()?;
                    if rtl.replace(b).is_some() {
                        self.error("E0104", item_start, "component has two `rtl` bodies");
                    }
                } else if self.at_word("model") && self.nth_is_punct(1, Punct::LBrace) {
                    self.advance();
                    let b = self.parse_block()?;
                    if model.replace(b).is_some() {
                        self.error("E0104", item_start, "component has two `model` bodies");
                    }
                } else {
                    return Err(self.error_expected("`isa`, `ports`, `rtl` or `model`"));
                }
                Ok(())
            })();
            if result.is_err() {
                self.recover_in_block();
            }
            if self.pos == before && !self.at_punct(Punct::RBrace) {
                self.advance();
            }
        }
        self.expect_punct(Punct::RBrace)?;
        Ok(ComponentDecl {
            docs,
            name,
            generics,
            isa,
            ports,
            rtl,
            model,
            span: start.to(self.prev_span()),
        })
    }

    fn parse_ports(&mut self) -> PResult<Vec<Port>> {
        self.parse_delimited(Punct::LBrace, Punct::RBrace, &[Punct::Semi], |p| {
            let docs = p.take_docs();
            let start = p.cur_span();
            // `in` is a hard keyword (used by `for`), so it cannot come back
            // from `expect_ident`; map it to the word by hand.
            let kind_word = if p.at_kw(Keyword::In) {
                let span = p.advance().span;
                Ident {
                    name: "in".to_string(),
                    span,
                }
            } else {
                p.expect_ident()?
            };
            let (name, kind) = match kind_word.name.as_str() {
                "in" | "out" | "async" => {
                    let dir = match kind_word.name.as_str() {
                        "in" => PortDir::In,
                        "out" => PortDir::Out,
                        _ => {
                            p.expect_kw(Keyword::In)?;
                            PortDir::AsyncIn
                        }
                    };
                    let name = p.expect_ident()?;
                    p.expect_punct(Punct::Colon)?;
                    let ty = p.parse_type()?;
                    (name, PortKind::Wire { dir, ty })
                }
                "bus" => {
                    let name = p.expect_ident()?;
                    p.expect_punct(Punct::Colon)?;
                    let ty = p.parse_type()?;
                    (name, PortKind::Bus(ty))
                }
                "clock" => {
                    let name = p.expect_ident()?;
                    (name, PortKind::Clock)
                }
                _ => {
                    return Err(p.error(
                        "E0105",
                        kind_word.span,
                        format!(
                            "expected `in`, `out`, `async in`, `bus` or `clock`, found `{}`",
                            kind_word.name
                        ),
                    ))
                }
            };
            let span = start.to(p.prev_span());
            Ok(Port {
                docs,
                name,
                kind,
                span,
            })
        })
    }

    // -- bus -------------------------------------------------------------------

    fn parse_bus(&mut self, docs: Docs) -> PResult<BusDecl> {
        let start = self.expect_word("bus")?;
        let name = self.expect_ident()?;
        self.expect_punct(Punct::LBrace)?;
        let items = self.parse_items_until_rbrace(|p| p.parse_bus_item())?;
        Ok(BusDecl {
            docs,
            name,
            items,
            span: start.to(self.prev_span()),
        })
    }

    fn parse_bus_item(&mut self) -> PResult<BusItem> {
        let docs = self.take_docs();
        let start = self.cur_span();
        let word = match self.peek_kind() {
            TokenKind::Ident(w) => w.clone(),
            _ => return Err(self.error_expected("a bus item")),
        };
        match word.as_str() {
            "clock" => {
                self.advance();
                let n = self.expect_ident()?;
                self.expect_semi()?;
                Ok(BusItem::Clock(n))
            }
            "params" => {
                self.advance();
                let params =
                    self.parse_delimited(Punct::LBrace, Punct::RBrace, &[Punct::Semi], |p| {
                        let s = p.cur_span();
                        p.expect_kw(Keyword::Const)?;
                        let name = p.expect_ident()?;
                        p.expect_punct(Punct::Colon)?;
                        let kind = p.parse_type()?;
                        // Optional default: `= 20`. Kept out of the AST for now;
                        // consume and warn so the file still parses.
                        if p.eat_punct(Punct::Eq) {
                            let e = p.parse_expr()?;
                            p.warning(
                                "W0100",
                                e.span,
                                "bus parameter defaults are not yet supported and are ignored",
                            );
                        }
                        let span = s.to(p.prev_span());
                        Ok(GenericParam { name, kind, span })
                    })?;
                Ok(BusItem::Params(params))
            }
            "wires" => {
                self.advance();
                let wires =
                    self.parse_delimited(Punct::LBrace, Punct::RBrace, &[Punct::Semi], |p| {
                        let docs = p.take_docs();
                        let name = p.expect_ident()?;
                        p.expect_punct(Punct::Colon)?;
                        let ty = p.parse_type()?;
                        let mut attrs = Vec::new();
                        while p.at_ident() {
                            let a = p.expect_ident()?;
                            attrs.push(match a.name.as_str() {
                                "driven_by" => WireAttr::DrivenBy(p.expect_ident()?),
                                "shared" => WireAttr::Shared,
                                "wired_and" => WireAttr::WiredAnd,
                                "wired_or" => WireAttr::WiredOr,
                                "active_low" => WireAttr::ActiveLow,
                                _ => {
                                    let v = if p.at_punct(Punct::LParen) {
                                        p.advance();
                                        let e = p.parse_expr()?;
                                        p.expect_punct(Punct::RParen)?;
                                        Some(e)
                                    } else {
                                        None
                                    };
                                    WireAttr::Other(a, v)
                                }
                            });
                        }
                        let span = name.span.to(p.prev_span());
                        Ok(BusWire {
                            docs,
                            name,
                            ty,
                            attrs,
                            span,
                        })
                    })?;
                Ok(BusItem::Wires(wires))
            }
            "roles" => {
                self.advance();
                let roles = self.parse_delimited(
                    Punct::LBrace,
                    Punct::RBrace,
                    &[Punct::Semi, Punct::Comma],
                    |p| p.expect_ident(),
                )?;
                Ok(BusItem::Roles(roles))
            }
            "enum" => Ok(BusItem::Enum(self.parse_enum(docs, "enum")?)),
            "transaction" => {
                self.advance();
                let name = self.expect_ident()?;
                let params = self.parse_params()?;
                let ret = if self.eat_punct(Punct::RArrow) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                self.expect_punct(Punct::LBrace)?;
                let mut roles = Vec::new();
                while !self.at_punct(Punct::RBrace) {
                    if self.at_eof() {
                        return Err(self.error_expected("`}`"));
                    }
                    let rs = self.cur_span();
                    let role = self.expect_ident()?;
                    let body = self.parse_block()?;
                    roles.push(RoleBody {
                        role,
                        body,
                        span: rs.to(self.prev_span()),
                    });
                }
                self.expect_punct(Punct::RBrace)?;
                Ok(BusItem::Transaction(TransactionDecl {
                    docs,
                    name,
                    params,
                    ret,
                    roles,
                    span: start.to(self.prev_span()),
                }))
            }
            "signal" => {
                self.advance();
                let name = self.expect_ident()?;
                let params = if self.at_punct(Punct::LParen) {
                    self.parse_params()?
                } else {
                    Vec::new()
                };
                self.expect_punct(Punct::Eq)?;
                let value = self.parse_expr()?;
                self.expect_semi()?;
                Ok(BusItem::Signal(SignalDecl {
                    docs,
                    name,
                    params,
                    value,
                    span: start.to(self.prev_span()),
                }))
            }
            _ => Err(self.error_expected(
                "`clock`, `params`, `wires`, `roles`, `enum`, `transaction` or `signal`",
            )),
        }
    }

    // -- isa -------------------------------------------------------------------

    fn parse_isa(&mut self, docs: Docs) -> PResult<IsaDecl> {
        let start = self.expect_word("isa")?;
        let name = self.expect_ident()?;
        self.expect_punct(Punct::LBrace)?;
        let items = self.parse_items_until_rbrace(|p| p.parse_isa_item())?;
        Ok(IsaDecl {
            docs,
            name,
            items,
            span: start.to(self.prev_span()),
        })
    }

    fn parse_isa_item(&mut self) -> PResult<IsaItem> {
        let docs = self.take_docs();
        let start = self.cur_span();
        if self.at_kw(Keyword::Const) {
            return Ok(IsaItem::Const(self.parse_const(docs)?));
        }
        let word = match self.peek_kind() {
            TokenKind::Ident(w) => w.clone(),
            _ => return Err(self.error_expected("an isa item")),
        };
        match word.as_str() {
            "space" => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect_punct(Punct::Colon)?;
                self.expect_word("addr")?;
                let addr = self.parse_type()?;
                self.expect_punct(Punct::RArrow)?;
                let cell = self.parse_type()?;
                let mut attrs = Vec::new();
                while self.eat_punct(Punct::Comma) {
                    attrs.push(self.expect_ident()?);
                }
                self.expect_semi()?;
                Ok(IsaItem::Space(SpaceDecl {
                    docs,
                    name,
                    addr,
                    cell,
                    attrs,
                    span: start.to(self.prev_span()),
                }))
            }
            "register" => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect_punct(Punct::Colon)?;
                let ty = self.parse_type()?;
                let mut fields = Vec::new();
                let mut views = Vec::new();
                if self.at_punct(Punct::LBrace) {
                    self.advance();
                    while !self.at_punct(Punct::RBrace) {
                        if self.at_eof() {
                            return Err(self.error_expected("`}`"));
                        }
                        let fdocs = self.take_docs();
                        let fstart = self.cur_span();
                        if self.eat_word("view") {
                            let vname = self.expect_ident()?;
                            self.expect_punct(Punct::Colon)?;
                            let vty = self.parse_type()?;
                            let (mapping, _) = self.parse_field_inits(false)?;
                            self.eat_punct(Punct::Semi);
                            views.push(RegisterView {
                                docs: fdocs,
                                name: vname,
                                ty: vty,
                                mapping,
                                span: fstart.to(self.prev_span()),
                            });
                        } else {
                            let fname = self.expect_ident()?;
                            self.expect_punct(Punct::Colon)?;
                            let fty = self.parse_type()?;
                            self.expect_punct(Punct::At)?;
                            let offset = self.parse_expr()?;
                            self.expect_semi()?;
                            fields.push(RegisterField {
                                docs: fdocs,
                                name: fname,
                                ty: fty,
                                offset,
                                span: fstart.to(self.prev_span()),
                            });
                        }
                    }
                    self.expect_punct(Punct::RBrace)?;
                } else {
                    self.expect_semi()?;
                }
                Ok(IsaItem::Register(RegisterDecl {
                    docs,
                    name,
                    ty,
                    fields,
                    views,
                    span: start.to(self.prev_span()),
                }))
            }
            "mode" => Ok(IsaItem::Mode(self.parse_enum(docs, "mode")?)),
            "enum" => Ok(IsaItem::Enum(self.parse_enum(docs, "enum")?)),
            "bundle" => Ok(IsaItem::Bundle(self.parse_bundle(docs)?)),
            "fault" => {
                self.advance();
                let name = self.expect_ident()?;
                let params = if self.at_punct(Punct::LParen) {
                    self.parse_params()?
                } else {
                    Vec::new()
                };
                let (attrs, _) = self.parse_field_inits(false)?;
                Ok(IsaItem::Fault(FaultDecl {
                    docs,
                    name,
                    params,
                    attrs,
                    span: start.to(self.prev_span()),
                }))
            }
            "fn" => {
                self.advance();
                let name = self.expect_ident()?;
                let params = self.parse_params()?;
                let ret = if self.eat_punct(Punct::RArrow) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                let body = self.parse_block()?;
                Ok(IsaItem::Fn(FnDecl {
                    docs,
                    name,
                    params,
                    ret,
                    body,
                    span: start.to(self.prev_span()),
                }))
            }
            "operand" => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect_punct(Punct::LBrace)?;
                let mut size = None;
                let mut fields = Vec::new();
                let mut tail = None;
                while !self.at_punct(Punct::RBrace) {
                    if self.at_eof() {
                        return Err(self.error_expected("`}`"));
                    }
                    if self.eat_word("size") {
                        size = Some(self.parse_expr()?);
                        self.expect_semi()?;
                    } else if self.eat_word("fields") {
                        fields = self.parse_delimited(Punct::LBrace, Punct::RBrace, &[Punct::Semi], |p| {
                            let fname = p.expect_ident()?;
                            p.expect_punct(Punct::Colon)?;
                            let fty = p.parse_type()?;
                            p.expect_punct(Punct::At)?;
                            let offset = p.parse_expr()?;
                            let span = fname.span.to(p.prev_span());
                            Ok(OperandField {
                                name: fname,
                                ty: fty,
                                offset,
                                span,
                            })
                        })?;
                    } else if self.eat_word("tail") {
                        tail = Some(self.parse_block()?);
                    } else {
                        return Err(self.error_expected("`size`, `fields` or `tail`"));
                    }
                }
                self.expect_punct(Punct::RBrace)?;
                Ok(IsaItem::Operand(OperandDecl {
                    docs,
                    name,
                    size,
                    fields,
                    tail,
                    span: start.to(self.prev_span()),
                }))
            }
            "istream" => {
                self.advance();
                self.expect_punct(Punct::LBrace)?;
                let mut space = None;
                let mut address = None;
                let mut next = None;
                let mut undefined = None;
                while !self.at_punct(Punct::RBrace) {
                    if self.at_eof() {
                        return Err(self.error_expected("`}`"));
                    }
                    let key = self.expect_ident()?;
                    match key.name.as_str() {
                        "space" => space = Some(self.expect_ident()?),
                        "address" => address = Some(self.parse_path_colons()?),
                        "next" => next = Some(self.parse_path_colons()?),
                        "undefined" => undefined = Some(self.parse_path_colons()?),
                        other => {
                            return Err(self.error(
                                "E0106",
                                key.span,
                                format!("unknown `istream` item `{other}`; expected `space`, `address`, `next` or `undefined`"),
                            ))
                        }
                    }
                    self.expect_semi()?;
                }
                self.expect_punct(Punct::RBrace)?;
                let (Some(space), Some(address), Some(next)) = (space, address, next) else {
                    return Err(self.error("E0106", start, "`istream` needs `space`, `address` and `next`"));
                };
                Ok(IsaItem::Istream(IstreamDecl {
                    space,
                    address,
                    next,
                    undefined,
                    span: start.to(self.prev_span()),
                }))
            }
            "latch" => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect_punct(Punct::Colon)?;
                let ty = self.parse_type()?;
                self.expect_punct(Punct::Eq)?;
                let init = self.parse_expr()?;
                self.expect_semi()?;
                Ok(IsaItem::Latch(LatchDecl {
                    docs,
                    name,
                    ty,
                    init,
                    span: start.to(self.prev_span()),
                }))
            }
            "operand_kind" => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect_punct(Punct::Colon)?;
                let ty = self.parse_type()?;
                let from = if self.eat_word("from") {
                    let binding = self.expect_ident()?;
                    self.expect_punct(Punct::Colon)?;
                    Some((binding, self.parse_path_colons()?))
                } else {
                    None
                };
                self.expect_punct(Punct::LBrace)?;
                let mut rules = Vec::new();
                while !self.at_punct(Punct::RBrace) {
                    if self.at_eof() {
                        return Err(self.error_expected("`}`"));
                    }
                    let rs = self.cur_span();
                    let rname = self.expect_ident()?;
                    let params = if self.at_punct(Punct::LParen) {
                        self.parse_params()?
                    } else {
                        Vec::new()
                    };
                    self.expect_punct(Punct::FatArrow)?;
                    let body = if self.at_punct(Punct::LBrace) {
                        ArmBody::Block(self.parse_block()?)
                    } else {
                        ArmBody::Expr(self.parse_expr()?)
                    };
                    self.expect_semi()?;
                    rules.push(KindRule {
                        name: rname,
                        params,
                        body,
                        span: rs.to(self.prev_span()),
                    });
                }
                self.expect_punct(Punct::RBrace)?;
                Ok(IsaItem::OperandKind(OperandKindDecl {
                    docs,
                    name,
                    ty,
                    from,
                    rules,
                    span: start.to(self.prev_span()),
                }))
            }
            "instruction" | "prefix" => {
                let prefix = word == "prefix";
                self.advance();
                let name = self.expect_ident()?;
                self.expect_punct(Punct::LBrace)?;
                let items = self.parse_items_until_rbrace(|p| p.parse_instruction_item())?;
                Ok(IsaItem::Instruction(InstructionDecl {
                    docs,
                    prefix,
                    name,
                    items,
                    span: start.to(self.prev_span()),
                }))
            }
            _ => Err(self.error_expected(
                "`space`, `register`, `mode`, `fault`, `fn`, `operand`, `operand_kind`, `instruction`, `prefix`, `latch`, `istream`, `enum`, `bundle` or `const`",
            )),
        }
    }

    fn parse_instruction_item(&mut self) -> PResult<InstructionItem> {
        let _docs = self.take_docs();
        let key = self.expect_ident()?;
        let item = match key.name.as_str() {
            "mnemonic" => {
                let e = self.parse_expr()?;
                self.expect_semi()?;
                InstructionItem::Mnemonic(e)
            }
            "encoding" => {
                let items =
                    self.parse_delimited(Punct::LBrace, Punct::RBrace, &[Punct::Semi], |p| {
                        let name = p.expect_ident()?;
                        let mut arg = None;
                        let mut fields = Vec::new();
                        if p.at_punct(Punct::LBrace) {
                            fields = p.parse_field_inits(false)?.0;
                        } else if !p.at_punct(Punct::Semi) && !p.at_punct(Punct::RBrace) {
                            arg = Some(p.parse_expr()?);
                            if p.at_punct(Punct::LBrace) {
                                fields = p.parse_field_inits(false)?.0;
                            }
                        }
                        let span = name.span.to(p.prev_span());
                        Ok(EncodingItem {
                            name,
                            arg,
                            fields,
                            span,
                        })
                    })?;
                InstructionItem::Encoding(items)
            }
            "operands" => {
                let ops =
                    self.parse_delimited(Punct::LParen, Punct::RParen, &[Punct::Comma], |p| {
                        let name = p.expect_ident()?;
                        p.expect_punct(Punct::Colon)?;
                        let kind = p.parse_path_colons()?;
                        let span = name.span.to(kind.span);
                        Ok(OperandBinding { name, kind, span })
                    })?;
                self.expect_semi()?;
                InstructionItem::Operands(ops)
            }
            "flags" => {
                let sets = self.parse_delimited(
                    Punct::LBrace,
                    Punct::RBrace,
                    &[Punct::Semi, Punct::Comma],
                    |p| {
                        let name = p.expect_ident()?;
                        p.expect_punct(Punct::Colon)?;
                        let entries = p.parse_delimited(
                            Punct::LBracket,
                            Punct::RBracket,
                            &[Punct::Comma],
                            |p| {
                                let flag = p.parse_path_dotted()?;
                                let cond = if p.eat_kw(Keyword::If) {
                                    Some(p.parse_expr()?)
                                } else {
                                    None
                                };
                                let span = flag.span.to(p.prev_span());
                                Ok(FlagEntry { flag, cond, span })
                            },
                        )?;
                        let span = name.span.to(p.prev_span());
                        Ok(FlagSet {
                            name,
                            entries,
                            span,
                        })
                    },
                )?;
                InstructionItem::Flags(sets)
            }
            "semantics" => InstructionItem::Semantics(self.parse_block()?),
            "timing" => InstructionItem::Timing(self.parse_field_inits(false)?.0),
            "implements" => {
                let p = self.parse_path_colons()?;
                self.expect_semi()?;
                InstructionItem::Implements(p)
            }
            "references" => {
                let refs =
                    self.parse_delimited(Punct::LBracket, Punct::RBracket, &[Punct::Comma], |p| {
                        p.expect_str()
                    })?;
                self.expect_semi()?;
                InstructionItem::References(refs)
            }
            "allowed_in" => {
                let modes =
                    self.parse_delimited(Punct::LBracket, Punct::RBracket, &[Punct::Comma], |p| {
                        p.parse_expr()
                    })?;
                self.expect_semi()?;
                InstructionItem::AllowedIn(modes)
            }
            "requires" => {
                let e = self.parse_expr()?;
                self.expect_semi()?;
                InstructionItem::Requires(e)
            }
            _ => {
                let v = if self.at_punct(Punct::Semi) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.expect_semi()?;
                InstructionItem::Other(key, v)
            }
        };
        Ok(item)
    }

    // -- system ------------------------------------------------------------------

    fn parse_system(&mut self, docs: Docs) -> PResult<SystemDecl> {
        let start = self.expect_word("system")?;
        let name = self.expect_ident()?;
        self.expect_punct(Punct::LBrace)?;
        let items = self.parse_items_until_rbrace(|p| p.parse_system_item())?;
        Ok(SystemDecl {
            docs,
            name,
            items,
            span: start.to(self.prev_span()),
        })
    }

    fn parse_system_item(&mut self) -> PResult<SystemItem> {
        let docs = self.take_docs();
        let start = self.cur_span();
        let word = match self.peek_kind() {
            TokenKind::Ident(w) => w.clone(),
            _ => return Err(self.error_expected("a system item")),
        };
        match word.as_str() {
            "clock" => {
                self.advance();
                let name = self.expect_ident()?;
                let source = if self.eat_punct(Punct::Colon) {
                    ClockSource::Frequency(self.parse_expr()?)
                } else {
                    self.expect_punct(Punct::Eq)?;
                    ClockSource::Derived(self.parse_expr()?)
                };
                self.expect_semi()?;
                Ok(SystemItem::Clock(ClockDecl {
                    docs,
                    name,
                    source,
                    span: start.to(self.prev_span()),
                }))
            }
            "component" => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect_punct(Punct::Colon)?;
                let component = self.parse_type()?;
                let clocks = if self.eat_punct(Punct::At) {
                    let cs = self.cur_span();
                    let bindings = if self.at_punct(Punct::LParen) {
                        self.parse_args()?
                    } else {
                        let id = self.expect_ident()?;
                        vec![Arg {
                            name: None,
                            span: id.span,
                            value: Expr {
                                span: id.span,
                                kind: ExprKind::Path(Path::single(id)),
                            },
                        }]
                    };
                    Some(ClockBinding {
                        bindings,
                        span: cs.to(self.prev_span()),
                    })
                } else {
                    None
                };
                self.expect_semi()?;
                Ok(SystemItem::Component(ComponentInst {
                    docs,
                    name,
                    component,
                    clocks,
                    span: start.to(self.prev_span()),
                }))
            }
            "bus" => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect_punct(Punct::Colon)?;
                let protocol = self.parse_type()?;
                let clock = self.parse_clock_ref()?;
                let bindings =
                    self.parse_delimited(Punct::LBrace, Punct::RBrace, &[Punct::Semi], |p| {
                        p.parse_bus_binding()
                    })?;
                Ok(SystemItem::Bus(BusInst {
                    docs,
                    name,
                    protocol,
                    clock,
                    bindings,
                    span: start.to(self.prev_span()),
                }))
            }
            "wire" => {
                self.advance();
                let target = self.parse_path_dotted()?;
                self.expect_punct(Punct::Eq)?;
                let source = self.parse_expr()?;
                self.expect_semi()?;
                Ok(SystemItem::Wire {
                    target,
                    source,
                    span: start.to(self.prev_span()),
                })
            }
            "profile" => {
                self.advance();
                let name = self.expect_ident()?;
                let entries =
                    self.parse_delimited(Punct::LBrace, Punct::RBrace, &[Punct::Semi], |p| {
                        let target = p.parse_path_dotted()?;
                        p.expect_punct(Punct::Colon)?;
                        let level = p.expect_ident()?;
                        let span = target.span.to(level.span);
                        Ok(ProfileEntry {
                            target,
                            level,
                            span,
                        })
                    })?;
                Ok(SystemItem::Profile(ProfileDecl {
                    docs,
                    name,
                    entries,
                    span: start.to(self.prev_span()),
                }))
            }
            _ => Err(self.error_expected("`clock`, `component`, `bus`, `wire` or `profile`")),
        }
    }

    fn parse_bus_binding(&mut self) -> PResult<BusBinding> {
        let word = self.expect_ident()?;
        match word.name.as_str() {
            "master" => Ok(BusBinding::Master(self.parse_path_dotted()?)),
            "slave" => {
                let port = self.parse_path_dotted()?;
                let mut ranges = Vec::new();
                if self.eat_punct(Punct::At) {
                    loop {
                        let rs = self.cur_span();
                        let space = if self.at_ident()
                            && !self.nth_is_punct(1, Punct::DotDot)
                            && !self.nth_is_punct(1, Punct::DotDotEq)
                        {
                            Some(self.expect_ident()?)
                        } else {
                            None
                        };
                        let range = self.parse_expr()?;
                        ranges.push(AddressRange {
                            space,
                            range,
                            span: rs.to(self.prev_span()),
                        });
                        if !self.eat_punct(Punct::Comma) {
                            break;
                        }
                    }
                }
                Ok(BusBinding::Slave { port, ranges })
            }
            "bridge" => Ok(BusBinding::Bridge(self.parse_path_dotted()?)),
            _ => {
                // `irq 0 = pit.out0;` and other named lines.
                let index = self.parse_postfix()?;
                self.expect_punct(Punct::Eq)?;
                let source = self.parse_postfix()?;
                Ok(BusBinding::Line {
                    name: word,
                    index,
                    source,
                })
            }
        }
    }

    /// One item of a `vectors { ... }` block.
    fn parse_vectors_item(&mut self) -> PResult<VectorsItem> {
        let start = self.cur_span();
        let key = self.expect_ident()?;
        match key.name.as_str() {
            "format" => {
                let f = self.expect_ident()?;
                self.expect_semi()?;
                Ok(VectorsItem::Format(f))
            }
            "memory" => {
                let m = self.expect_ident()?;
                self.expect_semi()?;
                Ok(VectorsItem::Memory(m))
            }
            "reg" => {
                let name = self.expect_ident()?;
                if self.eat_punct(Punct::Eq) {
                    let place = self.parse_expr()?;
                    self.expect_semi()?;
                    return Ok(VectorsItem::Reg {
                        name,
                        place,
                        span: start.to(self.prev_span()),
                    });
                }
                self.expect_punct(Punct::LBrace)?;
                self.expect_word("load")?;
                self.expect_punct(Punct::FatArrow)?;
                let load = self.parse_stmt()?;
                self.expect_word("read")?;
                self.expect_punct(Punct::FatArrow)?;
                let read = self.parse_expr()?;
                self.expect_semi()?;
                self.expect_punct(Punct::RBrace)?;
                Ok(VectorsItem::RegCustom {
                    name,
                    load: Box::new(load),
                    read,
                    span: start.to(self.prev_span()),
                })
            }
            other => Err(self.error(
                "E0107",
                key.span,
                format!("unknown `vectors` item `{other}`; expected `format`, `memory` or `reg`"),
            )),
        }
    }
}
