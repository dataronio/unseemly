#![macro_use]

extern crate lalrpop_intern;

use std::fmt;
use std::string::String;

#[derive(PartialEq,Eq,Clone,Copy,Hash)]
pub struct Name {
    id: lalrpop_intern::InternedString
}

impl ::runtime::reify::Reifiable for Name {
    fn ty_name() -> Name { n("Name") }

    fn reify(&self) -> ::runtime::eval::Value {
        ::runtime::eval::Value::AbstractSyntax(::ast::Ast::Atom(*self))
    }

    fn reflect(v: &::runtime::eval::Value) -> Name {
        extract!((v) ::runtime::eval::Value::AbstractSyntax = (ref ast)
                          => ::core_forms::ast_to_name(ast))
    }
}

// only available on nightly:
// impl !Send for Name {}

impl Name {
    pub fn sp(self) -> String { self.id.to_string() }
}


// TODO: move to `ast_walk`
// TODO: this interner doesn't support `gensym`...

/// Special name for negative `ast_walk`ing
pub fn negative_ret_val() -> Name {
    Name { id: lalrpop_intern::intern("⋄") }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "«{}»", self.sp())
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.sp())
    }
}

impl Name {
    pub fn is(self, s: &str) -> bool {
        self.sp() == s
    }

    pub fn is_name(self, n: Name) -> bool {
        self.sp() == n.sp()
    }
}

pub fn n(s: &str) -> Name {
    Name{ id: lalrpop_intern::intern(s) }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ContainedName {
    spelling: String
}

impl ContainedName {
    pub fn from_name(n: Name) -> ContainedName {
        ContainedName {
            spelling: n.sp()
        }
    }
}
