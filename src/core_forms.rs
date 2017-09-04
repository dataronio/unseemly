// This virtual machine kills cyber-fascists.



// Core forms!
//
// This is the definition of Unseemly, the bizarre boiled-down programming language.
//
// Unseemly programs have expressions and types (and probably kinds, too).
//

use name::*;
use parse::{SynEnv, FormPat};
use parse::FormPat::*;
use form::{Form, simple_form, Positive, Negative};
use util::assoc::Assoc;
use ast::*;
use std::rc::Rc;
use ty::*;
use runtime::eval::*;
use beta::*;
use ast_walk::WalkRule::*;
use num::bigint::ToBigInt;
use core_type_forms::*; // type forms are kinda bulky

pub fn ast_to_name(ast: &Ast) -> Name {
    match *ast { Atom(n) => n, _ => { panic!("ICE: {:?} is not an atom", ast) } }
}
pub fn vr_to_name(ast: &Ast) -> Name {
    match *ast { VariableReference(n) => n, _ => { panic!("ICE: {:?} is not an atom", ast) } }
}

// A brief digression about types and syntax quotation...
// Expressions are "positive", and are traversed leaf-to-root in an environment, producing a type.
// Patterns are "negative", and are traversed root-to-leave from a type, producing an environment.
// (`match` and `lambda` are examples of interactions between expressions and patterns.)
// Syntax quotation and unquotation embeds expressions/patterns
//  (at a different phase, which matters suprisingly little)
//  inside expressions/patterns.
//
// This looks like:
//                     pattern outside | expression outside      <-- (provides context)
//                   --------------------------------------
// pattern inside    | ok              | needs annotation
// expression inside | bonus check     | ok
//
// Examples of needed annotation:
//
//   optimize_pat '[{Pat<[List<[Int]<]<} cons a b]'
// In this case, we need to know the type of the syntax quote,
//  but the pattern wants to know its type so that it can tell us its environment.
//
//   match stx { '[{Pat} 1 + 5 * ,[{Expr<[Nat]<} stx_num], ]' => ... }
// In this case (looking at the expression interpolation),
//  we need to know the type of the interpolated expression syntax (a pattern)
//   in order to type-synthesize the arithmetic.
//
//
// Examples of when we get to do a bonus typecheck:
//
//   match stx { '[{Expr} f x]' => ... }
// In this case, we can check that the type of the scrutinee
//  (which is the type of the syntax quotation pattern)
//   equals Expr<[ (whatever `f` returns) ]<.
//
//   optimize_expr '[{Expr} match stx { ,[{Pat} my_pat], => ... } ]'
// In this case (looking at the Pat interpolation),
//  we can check that the type of the quoted scrutinee is the same as
//   the type of `my_pat` (after peeling off its `Pat<[]<`).
//
// Note that it doesn't matter whether the boundary is a quotation or an unquotation!
// Like I said, the phase doesn't matter much.
//

// This form isn't part of any nt! Instead, it's inserted into nts by `quote`.

/// Generate an unquoting form.
fn unquote<Mode: ::ast_walk::WalkMode>(nt: Name, ctf: SynEnv, pos: bool) -> Rc<Form> {
    Rc::new(Form {
        name: n("unquote"), // maybe add the `nt` to the name?
        grammar:
            Rc::new(if pos {
                form_pat!([(delim ",[", "[", /*]]*/ (named "body", (call "expr")))])
            } else {
                form_pat!([(delim ",[", "[", /*]]*/ (named "body", (call "pat")))])
            }),
        type_compare: Positive(NotWalked), // this is not a type form
        synth_type:
            // imagine: ` '[{expr} .[a : int . ,[body], ]. ]' `
            if pos {
                Positive(
                    // suppose that this is an expr, and `body` has the type `expr <[string]<`:
                    cust_rc_box!( move | unquote_parts | {
                        let interpolate_type = try!(unquote_parts.get_res(&n("body")));
                        expect_ty_node!( (interpolate_type ; find_type(&ctf, "type_apply") ;
                                             &unquote_parts.this_ast)
                            apply_parts;
                            {
                                let got_nt = ast_to_name(
                                    apply_parts.get_leaf_or_panic(&n("type_name")));
                                if  got_nt != nt {
                                    ty_err!(NtInterpMismatch(got_nt, nt) at
                                        unquote_parts.get_term(&n("body")));
                                }
                                let args = apply_parts.get_rep_leaf_or_panic(&n("arg"));
                                if args.len() != 1 {
                                    panic!("Kind error: expected one argument, got {:?}", args)
                                }
                                Ok(Ty::new(args[0].clone()))
                })}))
            } else {
                Negative(
                    // suppose that this is a pat,
                    // and the whole thing has type `expr <[ [int -> string] ]<`
                    cust_rc_box!( move | _unquote_parts | {
                        panic!("")
                    })
                )
            },

            // Also, let's suppose that we have something like:
            //   let some_pattern : pat <[int]< = ...
            //   let s = '[{pat} struct { a: ,[ some_pattern ],  b: b_var} ]'
            // ...what then?
        eval: // should be both
            Positive(cust_rc_box!( move | _unquote_parts | {
                panic!("");
            })),
        quasiquote: //should be both
            Positive(cust_rc_box!( move | _unquote_parts | {
                panic!("");
            }))
    })
}


/*
fn eval_quoted_stx(a: Ast, env: Assoc<Name, Value>) -> Ast {
    match a {
        Trivial | Atom(_) | VariableReference(_)
    }
}
*/

/// Remove an `ExtendEnv` without respecting its binding behavior.
/// This is safe if directly inside a `Node` that was just freshened.
/// (TODO: think about what "just" means here. It's super-subtle!)
pub fn strip_ee(a: &Ast) -> &Ast {
    match a { &ExtendEnv(ref body, _) => (&**body), _ => panic!("ICE: malformed thing") }
}

/// This is the Unseemly language.
pub fn make_core_syn_env() -> SynEnv {

    let ctf: SynEnv = make_core_syn_env_types();

    // This seems to be necessary to get separate `Rc`s into the closures.
    let ctf_0 = ctf.clone();
    let ctf_2 = ctf.clone();
    let ctf_3 = ctf.clone();
    let ctf_4 = ctf.clone();
    let ctf_5 = ctf.clone();
    let ctf_6 = ctf.clone();
    let ctf_7 = ctf.clone();
    let ctf_8 = ctf.clone();

    // Unseemly expressions
    let main_expr_forms = forms_to_form_pat![
        typed_form!("lambda",
            /* syntax */ /* TODO: add comma separators to the syntax! */
            (delim ".[", "[", /*]]*/ [
                               (star [(named "param", aat), (lit ":"),
                                      (named "p_t", (call "type"))]), (lit "."),
                (named "body",
                    (import [* ["param" : "p_t"]], (call "expr")))]),
            /* type */
            cust_rc_box!( move | part_types | {
                let lambda_type : Ty =
                    ty!({ find_type(&ctf_0, "fn") ;
                         "param" => [* part_types =>("param") part_types :
                                       (, try!(part_types.get_res(&n("p_t"))).concrete() )],
                         "ret" => (, try!(part_types.get_res(&n("body"))).concrete() )});
                Ok(lambda_type)}),
            /* evaluation */
            cust_rc_box!( move | part_values | {
                Ok(Function(Rc::new(Closure {
                    body: strip_ee(part_values.get_term_ref(&n("body"))).clone(),
                    params:
                    part_values.get_rep_term(&n("param")).iter().map(ast_to_name).collect(),
                    env: part_values.env
                })))
            })),

        typed_form!("apply", /* function application*/
            (delim "(", "(", /*))*/ [(named "rator", (call "expr")),
             (star (named "rand", (call "expr")))]),
            cust_rc_box!(move | part_types | {
                use ::ast_walk::WalkMode;
                let return_type = ::ty_compare::Subtype::underspecified(n("<return_type>"));
                let _ = try!(::ty_compare::must_subtype(
                    &ty!({ "type" "fn" :
                        "param" => (,seq try!(part_types.get_rep_res(&n("rand")))
                             .iter().map(|t| t.concrete()).collect::<Vec<_>>() ),
                        "ret" => (, return_type.concrete() )}),
                    &try!(part_types.get_res(&n("rator"))),
                    part_types.env.clone())
                        .map_err(|e| ::util::err::sp(e, part_types.this_ast.clone())));

                ::ty_compare::unification.with(|unif| { //TODO: try removing the `resolve`
                    Ok(::ty_compare::resolve(return_type, &part_types.env, &unif.borrow()))
                })
            }),
            cust_rc_box!( move | part_values | {
                match try!(part_values.get_res(&n("rator"))) {
                    Function(clos) => {
                        let mut new_env = clos.env.clone();
                        for (p, v) in clos.params.iter().zip(
                                try!(part_values.get_rep_res(&n("rand")))) {
                            new_env = new_env.set(*p, v);
                        }

                        ::runtime::eval::eval(&clos.body, new_env)
                    },
                    BuiltInFunction(::runtime::eval::BIF(f)) => {
                        Ok(f(try!(part_values.get_rep_res(&n("rand")))))
                    }
                    other => {
                        panic!("Type soundness bug: attempted to invoke {:?}
                        as if it were a function", other)
                    }
                }
            })),
        typed_form!("match",
            [(lit "match"), (named "scrutinee", (call "expr")),
             (delim "{", "{",
                 (plus [(named "p", (call "pat")), (lit "=>"),
                        (named "arm", (import ["p" = "scrutinee"], (call "expr")))]))],
            /* Typesynth: */
            cust_rc_box!(move | part_types | {
                let mut res : Option<Ty> = None;

                for arm_part_types in part_types.march_parts(&[n("arm")]) {
                    // We don't need to manually typecheck
                    //  that the arm patterns match the scrutinee;
                    //  the import handles that for us.

                    let arm_res = try!(arm_part_types.get_res(&n("arm")));

                    match res {
                        None => { res = Some(arm_res) }
                        Some(ref old_res) => {
                            ty_exp!(old_res, &arm_res, arm_part_types.get_term(&n("arm")));
                        }
                    }
                }
                match res {
                    None => { // TODO: this isn't anywhere near exhaustive
                        ty_err!(NonExhaustiveMatch(part_types.get_res(&n("scrutinee")).unwrap())
                            at Trivial /* TODO */)
                    },
                    Some(ty_res) => return Ok(ty_res)
                }
            }),
            /* Evaluation: */
            cust_rc_box!( move | part_values | {
                for arm_values in part_values.march_all(&[n("arm")]) {
                    // TODO: don't we need to set a context?
                    match arm_values.get_res(&n("arm")) {
                        Ok(res) => { return Ok(res); }
                        Err(()) => { /* try the next one */ }
                    }
                }
                panic!("No arms matched! This ought to be a type error, but isn't.");
            })
        ),
        /* Note that we inconveniently require the user to specify the type.
           "real" languages infer the type from the (required-to-be-unique)
           component name. */
        typed_form!("enum_expr",
             [(delim "+[", "[", /*]]*/ [(named "name", aat),
                                       (star (named "component", (call "expr")))]),
              (lit ":"), (named "t", (call "type"))],
            /* Typesynth: */
            cust_rc_box!( move | part_types | {
                let res : Ty = try!(part_types.get_res(&n("t")));
                expect_ty_node!( (res ; find_type(&ctf_2, "enum") ; &part_types.this_ast)
                    enum_type_parts;
                    {
                        for enum_type_part in enum_type_parts.march_all(&[n("name")]) {
                            if &part_types.get_term(&n("name"))
                                    != enum_type_part.get_leaf_or_panic(&n("name")) {
                                continue; // not the right arm
                            }

                            let component_types : Vec<Ty> =
                                enum_type_part.get_rep_leaf_or_panic(&n("component"))
                                    .iter().map(|a| Ty::new((*a).clone())).collect();

                            // TODO: check that they're the same length!

                            for (t, expected_t) in try!(part_types.get_rep_res(&n("component")))
                                    .iter().zip(component_types) {
                                ty_exp!(t, &expected_t, part_types.this_ast);
                            }

                            return Ok(res.clone());
                        }

                        ty_err!(NonexistentEnumArm
                                    (ast_to_name(&part_types.get_term(&n("name"))), res)
                                at part_types.this_ast);
                    }
                )
            }),
            /* Evaluate: */
            cust_rc_box!( move | part_values | {
                Ok(Enum(ast_to_name(&part_values.get_term(&n("name"))),
                    try!(part_values.get_rep_res(&n("component")))))
            })),
        typed_form!("struct_expr",
            (delim "*[", "[", /*]]*/
                (star [(named "component_name", aat), (lit ":"),
                       (named "component", (call "expr"))])),
            cust_rc_box!( move | part_types | {
                Ok(ty!({ find_type(&ctf_3, "struct") ;
                    "component_name" => (@"c" ,seq part_types.get_rep_term(&n("component_name"))),
                    "component" => (@"c" ,seq try!(part_types.get_rep_res(&n("component")))
                        .into_iter().map(|c : Ty| c.concrete()))
                }))
            }),
            cust_rc_box!( move | part_values | {
                let mut res = Assoc::new();

                for component_parts in part_values.march_parts(&[n("component")]) {
                    res = res.set(ast_to_name(&component_parts.get_term(&n("component_name"))),
                                  try!(component_parts.get_res(&n("component"))));
                }

                Ok(Struct(res))
            })),

        /* e.g.
         * let_type
         *   pair = mu lhs rhs. {l: lhs, r: rhs}
         *   point = pair <[int, int]<
         * in ...
         */
        typed_form!("let_type",
            [(lit "let_type"),
             (named "type_kind_stx", (anyways "*")),
             (import [* ["type_name" = "type_def"]], // put `pair` and `point` in type env
                 (star [(named "type_name", aat), (lit "="), (named "type_def", (call "type"))])),
             (lit "in"),
             (named "body", (import [* ["type_name" = "type_def"]], (call "expr")))],
            Body(n("body")),
            Body(n("body"))),

        /* e.g. where List = ∀ X. μ List. enum { Nil(), Cons(X, List<[X]<) }
         * .[x : List <[X]<  . match (unfold x) ... ].
         * (unfold is needed because `match` wants an `enum`, not a `μ`)
         * Exposes the inside of a μ type by performing one level of substitution.
         */
        typed_form!("unfold",
            [(lit "unfold"), (named "body", (call "expr"))],
            cust_rc_box!( move |unfold_parts| {
                // TODO: this "evaluates" types twice; once in `get_res` and once in `synth_type`
                // It shouldn't be necessary, and it's probably quadratic.
                // Maybe this points to a weakness in the LiteralLike approach to traversing types?
                let mu_typed = try!(unfold_parts.get_res(&n("body")));

                // Pull off the `mu` (and the `ExtendEnv` that it carries):
                // (This is sound because `mu`'s param must already be in the environment.)
                expect_ty_node!( (mu_typed.clone() ; find_type(&ctf_4, "mu_type") ;
                                    &unfold_parts.this_ast)
                    mu_parts;
                    {
                        // This acts like the `mu` was never there (and hiding the binding)
                        if let &ExtendEnv(ref body, _) = mu_parts.get_leaf_or_panic(&n("body")) {
                            let cur_env = unfold_parts.env.map(&|x: &Ty| x.concrete());
                            synth_type(&::alpha::substitute(body, &cur_env),
                                       unfold_parts.env.clone())
                        } else { panic!("ICE: no protection to remove!"); }
                    })
            }),
            Body(n("body"))),

        /* e.g. where List = ∀ X. μ List. enum { Nil (), Cons (X, List<[X]<) }
         * (.[x : List <[X]< . ...]. (fold +[Nil]+) ) : List<[X]<
         */
        typed_form!("fold",
            [(lit "fold"), (named "body", (call "expr")), (lit ":"), (named "t", (call "type"))],
            cust_rc_box!( move |fold_parts| {
                let goal_type = try!(fold_parts.get_res(&n("t")));
                // TODO: I can't figure out how to pull this out into a function
                //  to invoke both here and above, since `mu_type_0` needs cloning...
                let folded_goal = expect_ty_node!(
                        (goal_type.clone() ; find_type(&ctf_5, "mu_type") ; &fold_parts.this_ast)
                    mu_parts;
                    {
                        // This acts like the `mu` was never there (and hiding the binding)
                        if let &ExtendEnv(ref body, _) = mu_parts.get_leaf_or_panic(&n("body")) {
                            let cur_env = fold_parts.env.map(&|x: &Ty| x.concrete());
                            try!(synth_type(&::alpha::substitute(body, &cur_env),
                                            fold_parts.env.clone()))
                        } else { panic!("ICE: no protection to remove!"); }
                    });

                ty_exp!(&try!(fold_parts.get_res(&n("body"))), &folded_goal,
                        fold_parts.this_ast);
                Ok(goal_type)
            }),
            Body(n("body"))),

        typed_form!("forall_expr",
            [(lit "forall"), (star (named "param", aat)), (lit "."),
             (named "body", (import [* [forall "param"]], (call "expr")))],
            cust_rc_box!( move |forall_parts| {
                Ok(ty!({"type" "forall_type" :
                    "param" => (,seq forall_parts.get_rep_term(&n("param"))),
                    "body" => (import [* [forall "param"]]
                        (, try!(forall_parts.get_res(&n("body"))).concrete()))
                }))
            }),
            Body(n("body"))),


        typed_form!("quote",
            [(delim "`[", "[", /*]]*/ [/* TODO, maybe after the parser is improved */])],
            cust_rc_box!( move | quote_parts | {
                //TODO put variables in phases!!! !!!!! !!!!!!!!!!!!
                Ok(ty!({ find_type(&ctf_8, "type_apply") ;
                    "type_name" => (, quote_parts.get_term(&n("nt")) ),
                    "arg" => [ (, try!(quote_parts.get_res(&n("body"))).concrete() )]
                }))
            }),
            cust_rc_box!( move | _quote_values | {
                panic!("TODO")
                // Traverse the body, form
            })
        )

        // The first use for syntax quotes will be in macro definitions.
        // But we will someday need them as expressions.
    ];


    let main_pat_forms = forms_to_form_pat_export![
        negative_typed_form!("enum_pat",
            (delim "+[", "[", /*]]*/ [(named "name", aat),
                                      (star (named "component", (call "pat")))]),
            /* (Negatively) Typecheck: */
            cust_rc_box!( move | part_types |
                expect_ty_node!( (part_types.context_elt() ; find_type(&ctf_6, "enum") ;
                                      &part_types.this_ast)
                    enum_type_parts;
                    {
                        let arm_name = &part_types.get_term(&n("name"));

                        for enum_type_part in enum_type_parts.march_all(&[n("name")]) {

                            if arm_name != enum_type_part.get_leaf_or_panic(&n("name")) {
                                continue; // not the right arm
                            }

                            let component_types : Vec<Ty> =
                                enum_type_part.get_rep_leaf_or_panic(&n("component"))
                                    .iter().map(|a| Ty::new((*a).clone())).collect();

                            let mut res = Assoc::new();
                            for sub_res in &try!(part_types
                                    .get_rep_res_with(&n("component"), component_types)) {
                                res = res.set_assoc(sub_res);
                            }

                            return Ok(res);
                        }
                        ty_err!(NonexistentEnumArm(ast_to_name(arm_name),
                            Ty::new(Trivial)) /* TODO `LazyWalkReses` needs more information */
                            at arm_name.clone())
                }
            )),
            /* (Negatively) Evaluate: */
            cust_rc_box!( move | part_values | {
                match *part_values.context_elt() /* : Value */ {
                    Enum(ref name, ref elts) => {
                        // "Try another branch"
                        if name != &ast_to_name(&part_values.get_term(&n("name"))) {
                            return Err(());
                        }

                        let mut res = Assoc::new();
                        for sub_res in &try!(part_values.get_rep_res_with(&n("component"),
                                                                          elts.clone())) {
                            res = res.set_assoc(sub_res);
                        }

                        Ok(res)
                    }
                    _ => panic!("Type ICE: non-enum")
                }
            })) => [* ["component"]],
        negative_typed_form!("struct_pat",
            [(delim "*[", "[", /*]]*/
                 (star [(named "component_name", aat), (lit ":"),
                        (named "component", (call "pat"))]))],
            /* (Negatively) typesynth: */
            cust_rc_box!( move | part_types |
                expect_ty_node!( (part_types.context_elt() ; find_type(&ctf_7, "struct") ;
                                      &part_types.this_ast)
                    struct_type_parts;
                    {
                        let mut res = Assoc::new();
                        for component_ctx in part_types.march_parts(&[n("component")]) {
                            let mut component_found = false;
                            for struct_type_part
                                    in struct_type_parts.march_all(&[n("component")]) {
                                if &component_ctx.get_term(&n("component_name"))
                                    != struct_type_part.get_leaf_or_panic(&n("component_name")) {
                                    continue;
                                }
                                component_found = true;

                                let component_type = Ty::new(
                                    struct_type_part.get_leaf_or_panic(&n("component")).clone());
                                res = res.set_assoc(
                                    &try!(component_ctx.with_context(component_type)
                                        .get_res(&n("component"))));
                                break;
                            }
                            if !component_found {
                                ty_err!(NonexistentStructField(
                                        ast_to_name(&component_ctx.get_term(&n("component_name"))),
                                        part_types.context_elt().clone())
                                    at part_types.get_rep_term(&n("component"))[0].clone());
                            }
                        }
                        Ok(res)
                    })),
            cust_rc_box!( move | part_values | {
                match *part_values.context_elt() {
                    Struct(ref contents) => {
                        let mut res = Assoc::new();

                        for component_ctx in part_values.march_parts(&[n("component")]) {
                            res = res.set_assoc(
                                &try!(component_ctx
                                    .with_context(contents.find_or_panic(
                                        &ast_to_name(
                                            &component_ctx.get_term(&n("component_name"))))
                                            .clone())
                                    .get_res(&n("component"))));
                        }

                        Ok(res)
                    }
                    _ => panic!("Type ICE: non-struct")
                }
            }))  => [* ["component"]]];

    assoc_n!(
        "pat" => Rc::new(Biased(Rc::new(main_pat_forms), Rc::new(AnyAtomicToken))),
        "expr" => Rc::new(Biased(Rc::new(main_expr_forms), Rc::new(VarRef)))
    )
        .set_assoc(&ctf) /* throw in the types! */
}

/**
 * Mostly for testing purposes, this looks up forms by name.
 * In the "real world", programmers look up forms by syntax, using a parser.
 */
pub fn find_form(se: &SynEnv, nt: &str, form_name: &str) -> Rc<Form> {

    fn find_form_rec(f: &FormPat, form_name: &str) -> Option<Rc<Form>> {
        match *f {
            Scope(ref f, _) => {
                if f.name.is(form_name) {
                    Some(f.clone())
                } else {
                    None
                }
            }
            Alt(ref vf) => {
                for f in vf {
                    let res = find_form_rec(f, form_name);
                    if res.is_some() { return res; }
                }
                None
            }
            Biased(ref lhs, ref rhs) => {
                let l_res = find_form_rec(lhs, form_name);
                if l_res.is_some() { l_res } else { find_form_rec(rhs, form_name) }
            }
            _ => None,
        }
    }
    let pat = se.find_or_panic(&n(nt));

    find_form_rec(pat, form_name)
        .expect(format!("{:?} not found in {:?}", form_name, pat).as_str())
}

fn find_type(se: &SynEnv, form_name: &str) -> Rc<Form> {
    find_form(se, "type", form_name)
}

thread_local! {
    pub static core_forms: SynEnv = make_core_syn_env();
}

pub fn outermost_form() -> FormPat {
    Call(n("expr")) // `n` isn't static
}

pub fn find_core_form(nt: &str, name: &str) -> Rc<Form> {
    core_forms.with(|cf| find_form(cf, nt, name))
}

pub fn get_core_forms() -> SynEnv {
    core_forms.with(|cf| cf.clone())
}



#[test]
fn form_grammar() {
    let cse = make_core_syn_env();
    use read::*;
    use read::DelimChar::*;

    assert_eq!(::parse::parse(&form_pat!((call "type")),
                              &cse.clone(),
                              &tokens!([""; "Ident" "->" "Ident"])),
               Ok(ast!({ find_form(&cse, "type", "fn");
                   ["ret" => {find_form(&cse, "type", "Ident") ; []},
                    "param" => [{find_form(&cse, "type", "Ident") ; []}]]})));
}


#[test]
fn form_expect_node() {
    let ast = ast!({ find_core_form("expr", "apply");
        ["rand" => [(vr "f")], "rator" => (vr "x")]});
    let _: Result<(), ()> = expect_node!(
        ( ast ; find_core_form("expr", "apply")) env; //expect_f = "rand", expect_x = "rator";
        {
            assert_eq!(env.get_rep_leaf_or_panic(&n("rand")), vec![&ast!((vr "f"))]);
            assert_eq!(env.get_leaf_or_panic(&n("rator")), &ast!((vr "x")));
            Ok(())
        });
}

#[test]
fn form_type() {
    let simple_ty_env = assoc_n!(
        "X" => ty!({ find_core_form("type", "Int") ; }),
        "N" => ty!({ find_core_form("type", "Nat") ; }));

    let lam = find_core_form("expr", "lambda");
    let fun = find_core_form("type", "fn");


    assert_eq!(synth_type(&ast!( (vr "X") ), simple_ty_env.clone()),
               Ok(ty!( { find_core_form("type", "Int") ; })));

    assert_eq!(synth_type(&ast!(
        { lam.clone() ;
            "param" => [@"p" "y"],
            "p_t" => [@"p" { find_core_form("type", "Nat") ; }],
            "body" => (import [* [ "param" : "p_t" ]] (vr "X"))}),
        simple_ty_env.clone()),
        Ok(ty!({ fun.clone() ;
            "param" => [{ find_core_form("type", "Nat") ; }],
            "ret" => { find_core_form("type", "Int") ; }})));
}

#[test]
fn type_apply_with_subtype() { // Application can perform subtyping

    let nat_ty = ty!({ "type" "Nat" : });

    let ty_env = assoc_n!(
        "N" => nat_ty.clone(),
        "nat_to_nat" => ty!({ "type" "fn" :
            "param" => [ (, nat_ty.concrete() ) ],
            "ret" => (, nat_ty.concrete() )}),
        "∀t_t_to_t" => ty!({ "type" "forall_type" :
            "param" => ["T"],
            "body" => (import [* [forall "param"]]
                { "type" "fn" :
                    "param" => [ (vr "T") ],
                    "ret" => (vr "T") })}));

    assert_eq!(synth_type(&ast!(
            { "expr" "apply" : "rator" => (vr "nat_to_nat") , "rand" => [ (vr "N") ]}),
            ty_env.clone()),
        Ok(nat_ty.clone()));

    assert_eq!(synth_type(&ast!(
            { "expr" "apply" : "rator" => (vr "∀t_t_to_t") , "rand" => [ (vr "N") ]}),
            ty_env.clone()),
        Ok(nat_ty.clone()));

}

#[test]
fn form_eval() {
    let simple_env = assoc_n!("x" => val!(i 18),
                              "w" => val!(i 99),
                              "b" => val!(b false));

    assert_eq!(eval(&ast!((vr "x")), simple_env.clone()),
               Ok(Int(18.to_bigint().unwrap())));

    // (λy.w) x
    assert_eq!(eval(&ast!(
        { "expr" "apply" :
             "rator" =>
                { "expr" "lambda" :
                    "param" => [@"p" "y"],
                    "p_t" => [@"p" "Int"],
                    "body" => (import [* [ "param" : "p_t" ]]  (vr "w"))},
             "rand" => [(vr "x")]
            }),
        simple_env.clone()),
        Ok(Int(99.to_bigint().unwrap())));

    // (λy.y) x
    assert_eq!(eval(&ast!(
        { "expr" "apply" :
             "rator" =>
                { "expr" "lambda" :
                    "param" => [@"p" "y"],
                    "p_t" => [@"p" "Int"],
                    "body" => (import [* [ "param" : "p_t" ]]  (vr "y"))},
             "rand" => [(vr "x")]
            }),
        simple_env.clone()),
        Ok(Int(18.to_bigint().unwrap())));

}

#[test]
fn alg_type() {
    let mt_ty_env = Assoc::new();
    let simple_ty_env = assoc_n!(
        "x" => ty!("Int"), "b" => ty!("Bool"), "f" => ty!("Float"));

    let my_enum = ty!({ "type" "enum" :
        "name" => [@"c" "Adams", "Jefferson", "Burr"],
        "component" => [@"c" ["Int"], ["Int", "Bool"], ["Float", "Float"]]
    });

    // Typecheck enum pattern
    assert_eq!(neg_synth_type(&ast!(
        { "pat" "enum_pat" :
            "name" => "Jefferson",
            "component" => ["abc", "def"]
        }),
        mt_ty_env.set(negative_ret_val(), my_enum.clone())),
        Ok(Assoc::new().set(n("abc"), ty!("Int")).set(n("def"), ty!("Bool"))));

    // Typecheck enum expression
    assert_eq!(synth_type(&ast!(
        { "expr" "enum_expr" :
            "name" => "Jefferson",
            "component" => [(vr "x"), (vr "b")],
            "t" => (, my_enum.concrete() )
        }),
        simple_ty_env.clone()),
        Ok(my_enum.clone()));


    let my_struct = ty!({ "type" "struct" :
        "component_name" => [@"c" "x", "y"],
        "component" => [@"c" "Int", "Float"]
    });

    // Typecheck struct pattern
    assert_eq!(neg_synth_type(&ast!(
            { "pat" "struct_pat" :
                "component_name" => [@"c" "y", "x"],
                "component" => [@"c" "yy", "xx"]
            }),
            mt_ty_env.set(negative_ret_val(), my_struct.clone())),
        Ok(assoc_n!("yy" => ty!("Float"), "xx" => ty!("Int"))));

    // Typecheck struct expression

    // TODO: currently {x: integer, y: float} ≠ {y: float, x: integer}
    // Implement proper type equality!
    assert_eq!(synth_type(&ast!(
            { "expr" "struct_expr" :
                "component_name" => [@"c" "x", "y"],
                "component" => [@"c" (vr "x"), (vr "f")]
            }),
            simple_ty_env.clone()),
        Ok(my_struct));

    // Simple match...

    assert_eq!(synth_type(&ast!({ "expr" "match" :
                "scrutinee" => (vr "f"),
                "p" => [@"arm" "my_new_name", "unreachable"],
                "arm" => [@"arm" (import ["p" = "scrutinee"] (vr "my_new_name")),
                                 (import ["p" = "scrutinee"] (vr "f"))]
            }),
            simple_ty_env.clone()),
        Ok(ty!("Float")));

    assert_m!(synth_type(&ast!({ "expr" "match" :
            "scrutinee" => (vr "b"),
            "p" => [@"arm" "my_new_name", "unreachable"],
            "arm" => [@"arm" (import ["p" = "scrutinee"] (vr "my_new_name")),
                             (import ["p" = "scrutinee"] (vr "f"))]
        }),
        simple_ty_env.clone()),
        ty_err_p!(Mismatch(_,_)));

    assert_m!(synth_type(&ast!({ "expr" "match" :
                "scrutinee" => (vr "my_enum"),
                "p" => [@"arm" { "pat" "enum_pat" :
                    "name" => "Hamilton", "component" => ["ii"] // Never gonna be president...
                }],
                "arm" => [@"arm" (import ["p" = "scrutinee"] (vr "ii"))]
        }),
        simple_ty_env.set(n("my_enum"), my_enum.clone())),
        ty_err_p!(NonexistentEnumArm(_,_))
    );


    assert_eq!(synth_type(&ast!({ "expr" "match" :
                "scrutinee" => (vr "my_enum"),
                "p" => [@"arm"
                { "pat" "enum_pat" => [* ["component"]] :
                    "name" => "Adams", "component" => ["ii"]
                },
                { "pat" "enum_pat" => [* ["component"]] :
                    "name" => "Jefferson", "component" => ["ii", "bb"]
                },
                { "pat" "enum_pat" => [* ["component"]] :
                    "name" => "Burr", "component" => ["xx", "yy"]
                }],
                "arm" => [@"arm" (import ["p" = "scrutinee"] (vr "ii")),
                                 (import ["p" = "scrutinee"] (vr "ii")),
                                 (import ["p" = "scrutinee"] (vr "x"))]
        }),
        simple_ty_env.set(n("my_enum"), my_enum.clone())),
        Ok(ty!("Int")));
}

#[test]
fn alg_eval() {
    let cse = make_core_syn_env();

    let mt_env = Assoc::new();
    let simple_env = assoc_n!("x" => val!(i 18), "w" => val!(i 99), "b" => val!(b false));

    // Evaluate enum pattern
    assert_eq!(neg_eval(&ast!(
        { "pat" "enum_pat" => [* ["component"]] :
            "name" => "choice1",
            "component" => ["abc", "def"]
        }),
        mt_env.set(negative_ret_val(), val!(enum "choice1", (i 9006), (b true)))),
        Ok(assoc_n!("abc" => val!(i 9006), "def" => val!(b true))));

    assert_eq!(neg_eval(&ast!(
        { "pat" "enum_pat" => [* ["component"]] :
            "name" => "choice1",
            "component" => ["abc", "def"]
        }),
        mt_env.set(negative_ret_val(), val!(enum "choice0", (i 12321)))),
        Err(()));

    // Evaluate enum expression

    let enum_t = find_form(&cse, "type", "enum");

    let my_enum_t = ast!({ enum_t.clone() ;
        "name" => [@"c" "choice0", "choice1", "choice2"],
        "component" => [@"c" ["Int"], ["Int", "Bool"], ["Float", "Float"]]
    });

    let enum_e = find_form(&cse, "expr", "enum_expr");

    let choice1_e = ast!(
        { enum_e.clone() ;
            "name" => "choice1",
            "component" => [(vr "x"), (vr "b")],
            "t" => (, my_enum_t.clone())
        });

    assert_eq!(eval(&choice1_e, simple_env.clone()),
        Ok(val!(enum "choice1", (i 18), (b false))));

    // Evaluate struct pattern

    assert_eq!(neg_eval(&ast!(
        {  "pat" "struct_pat" => [* ["component"]] :
            "component_name" => [@"c" "x", "y"],
            "component" => [@"c" "xx", "yy"]
        }),
        mt_env.set(negative_ret_val(),
                   Struct(assoc_n!("x" => val!(i 0), "y" => val!(b true))))),
        Ok(assoc_n!("xx" => val!(i 0), "yy" => val!(b true))));

    // Evaluate match


    assert_eq!(eval(&ast!({ "expr" "match" :
                "scrutinee" => (vr "x"),
                "p" => [@"arm" "my_new_name", "unreachable"],
                "arm" => [@"arm" (import ["p" = "scrutinee"] (vr "my_new_name")),
                                 (import ["p" = "scrutinee"] (vr "x"))]
        }),
        simple_env.clone()),
        Ok(val!(i 18)));

    assert_eq!(eval(&ast!({ "expr" "match" :
                "scrutinee" => (, choice1_e),
                "p" => [@"arm"
                { "pat" "enum_pat" => [* ["component"]] :
                    "name" => "choice2", "component" => ["xx", "yy"]
                },
                { "pat" "enum_pat" => [* ["component"]] :
                    "name" => "choice1", "component" => ["ii", "bb"]
                },
                { "pat" "enum_pat" => [* ["component"]] :
                    "name" => "choice0", "component" => ["ii"]
                }],
                "arm" => [@"arm" (import ["p" = "scrutinee"] (vr "yy")),
                                 (import ["p" = "scrutinee"] (vr "bb")),
                                 (import ["p" = "scrutinee"] (vr "ii"))]
        }),
        simple_env.clone()),
        Ok(val!(b false)));
}


#[test]
fn recursive_types() {
    let int_list_ty =
        ty!( { "type" "mu_type" :
            "param" => ["IntList"],
            "body" => (import [* [prot "param"]] { "type" "enum" :
                "name" => [@"c" "Nil", "Cons"],
                "component" => [@"c" [], ["Int", (vr "IntList") ]]})});

    let ty_env = assoc_n!(
        "IntList" => int_list_ty.clone(),  // this is a type definition...
        "il_direct" => int_list_ty.clone()  // ...and this is a value with a type
        // TODO: ... distinguish between these in the environment! Is the difference ... kind?

        // We should never have `vr`s in the environment unless "protected" by a μ.
        // TODO: enforce that:
        //"il_named" => ty!((vr "IntList"))
    );

    // `IntList` shouldn't substitute
    assert_eq!(synth_type(&ast!((vr "il_direct")), ty_env.clone()), Ok(int_list_ty.clone()));

    // I don't want these tests to depend on alpha-equivalence, so just disable freshening here.
    without_freshening!{
    // Test that unfolding a type produces one that's "twice as large", minus the outer mu
    assert_eq!(synth_type(
        &ast!({"expr" "unfold" : "body" => (vr "il_direct")}), ty_env.clone()),
        Ok(ty!({ "type" "enum" :
                "name" => [@"c" "Nil", "Cons"],
                "component" => [@"c" [], ["Int", (, int_list_ty.concrete()) ]]})));

    // folding an unfolded thing should give us back the same thing
    assert_eq!(synth_type(
        &ast!( { find_core_form("expr", "fold") ;
            "body" => { find_core_form("expr", "unfold") ;
                "body" => (vr "il_direct") },
            "t" => (, int_list_ty.concrete() )}),
        ty_env.clone()),
        Ok(int_list_ty.clone()));

    // Unfold a type and then match it
    assert_eq!(synth_type(
        &ast!( { "expr" "match" :
            "scrutinee" => { "expr" "unfold" : "body" => (vr "il_direct") },
            "p" => [@"arm" { "pat" "enum_pat" => [* ["component"]] :
                "name" => "Cons",
                "component" => ["car", "cdr"],
                "t" => (vr "IntList")
            }],
            "arm" => [@"arm" (import ["p" = "scrutinee"] (vr "car"))]
        }),
        ty_env.clone()),
        Ok(ty!("Int")));

    // Unfold a type and then extract the part that should have the same type as the outer type
    assert_eq!(synth_type(
        &ast!( { "expr" "match" :
            "scrutinee" => { "expr" "unfold" : "body" => (vr "il_direct") },
            "p" => [@"arm" { "pat" "enum_pat" => [* ["component"]] :
                "name" => "Cons",
                "component" => ["car", "cdr"],
                "t" => (vr "IntList")
            }],
            "arm" => [@"arm" (import ["p" = "scrutinee"] (vr "cdr"))]
        }),
        ty_env.clone()),
        Ok(int_list_ty.clone())
    );
    };

    // Test that missing an unfold fails
    assert_m!(synth_type(
            &ast!( { "expr" "match" :
                "scrutinee" =>  (vr "il_direct") ,
                "p" => [@"arm" { "pat" "enum_pat" => [* ["component"]] :
                "name" => "Cons",
                "component" => ["car", "cdr"],
                "t" => (vr "IntList")
            }],
            "arm" => [@"arm" (import ["p" = "scrutinee"] (vr "car"))]
        }),
        ty_env.clone()),
        ty_err_p!(UnableToDestructure(_,name_enum)),
        name_enum == n("enum")
    );
}
