use crate::{
    ast::{
        ebnf_02::{Program, TopItem},
        ebnf_03::{ContextBlock, ContextItem},
        ebnf_04::FnDef,
        ebnf_05::HashBlock,
        ebnf_06::{NodeDecl, NodeKind, PropValue, Property},
        ebnf_07::{WireDecl, WireEndpoint},
        ebnf_08::{ArrangeMode, FlowDirection},
        ebnf_09::EventHandler,
        ebnf_10::{Duration, Effect, RerouteDir, RerouteEffect},
        ebnf_11::{BinOp, Expr},
    },
    lexer::duration_unit::DurationUnit,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §2  Program / TopItem
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_02 {
    use super::*;

    #[test]
    fn should_allow_empty_program() -> Result<(), String> {
        let prog = Program { items: vec![] };
        if !prog.items.is_empty() {
            return Err(format!("expected no items, got {}", prog.items.len()));
        }
        Ok(())
    }

    #[test]
    fn should_clone_program_independently_of_original() -> Result<(), String> {
        let original = Program {
            items: vec![TopItem::FnDef(FnDef {
                name: "f".into(),
                params: vec![],
                body: Expr::Integer(1),
            })],
        };
        let mut clone = original.clone();

        // Clearing the clone must not affect the original
        clone.items.clear();

        if original.items.is_empty() {
            return Err("original.items was emptied by a mutation of the clone".into());
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §3  ContextBlock / ContextItem
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_03 {
    use super::*;

    #[test]
    fn should_clone_context_block_independently_of_original() -> Result<(), String> {
        let original = ContextBlock {
            items: vec![ContextItem::WordSize(32)],
        };
        let mut clone = original.clone();
        clone.items.clear();

        if original.items.is_empty() {
            return Err("original.items was emptied by a mutation of the clone".into());
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §4  FnDef
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_04 {
    use super::*;

    #[test]
    fn should_allow_fn_def_with_no_params() -> Result<(), String> {
        let f = FnDef { name: "f".into(), params: vec![], body: Expr::Integer(0) };
        if !f.params.is_empty() {
            return Err(format!("expected no params, got {}", f.params.len()));
        }
        Ok(())
    }

    #[test]
    fn should_clone_fn_def_independently_of_original() -> Result<(), String> {
        let original = FnDef {
            name: "f".into(),
            params: vec!["x".into(), "y".into()],
            body: Expr::Integer(1),
        };
        let mut clone = original.clone();

        // Mutate the clone's name and param list
        clone.name = "g".into();
        clone.params.push("z".into());

        if original.name != "f" {
            return Err(format!(
                "original.name was changed by mutation of clone: expected \"f\", got {:?}",
                original.name
            ));
        }
        if original.params.len() != 2 {
            return Err(format!(
                "original.params.len() was changed by mutation of clone: expected 2, got {}",
                original.params.len()
            ));
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §5  HashBlock
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_05 {
    use super::*;

    #[test]
    fn should_clone_hash_block_preserving_name() -> Result<(), String> {
        let original = HashBlock { name: "SHA256".into(), items: vec![] };
        let mut clone = original.clone();
        clone.name = "SHA512".into();

        if original.name != "SHA256" {
            return Err(format!(
                "original.name was changed by mutation of clone: expected \"SHA256\", got {:?}",
                original.name
            ));
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §6  NodeDecl / NodeKind / Property / PropValue
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_06 {
    use super::*;

    #[test]
    fn should_allow_empty_node_property_list() -> Result<(), String> {
        let node = NodeDecl { name: "a".into(), kind: NodeKind::Register, properties: vec![] };
        if !node.properties.is_empty() {
            return Err(format!("expected no properties, got {}", node.properties.len()));
        }
        Ok(())
    }

    #[test]
    fn should_clone_node_decl_independently_of_original() -> Result<(), String> {
        let original = NodeDecl {
            name: "a".into(),
            kind: NodeKind::Register,
            properties: vec![Property {
                name: "label".into(),
                value: PropValue::Str("a".into()),
            }],
        };
        let mut clone = original.clone();
        clone.name = "b".into();
        clone.properties.clear();

        if original.name != "a" {
            return Err(format!(
                "original.name changed by mutation of clone: expected \"a\", got {:?}",
                original.name
            ));
        }
        if original.properties.is_empty() {
            return Err("original.properties was cleared by mutation of clone".into());
        }
        Ok(())
    }

    #[test]
    fn should_represent_user_defined_kind_with_its_name() -> Result<(), String> {
        let node = NodeDecl { name: "m".into(), kind: NodeKind::User("mux".into()), properties: vec![] };
        match node.kind {
            NodeKind::User(ref s) if s == "mux" => Ok(()),
            other => Err(format!("expected User(\"mux\"), got {other:?}")),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §7  WireDecl / WireEndpoint
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_07 {
    use super::*;

    #[test]
    fn should_clone_wire_decl_preserving_open_endpoint() -> Result<(), String> {
        let original = WireDecl {
            name: Some("carry".into()),
            source: WireEndpoint::Open,
            target: WireEndpoint::Node("a".into()),
        };
        let mut clone = original.clone();

        // Mutate the clone's name
        clone.name = None;

        if original.name.as_deref() != Some("carry") {
            return Err(format!(
                "original.name changed by mutation of clone: expected Some(\"carry\"), got {:?}",
                original.name
            ));
        }

        // Verify endpoint kinds are preserved in the clone
        match clone.source {
            WireEndpoint::Open => {}
            other => return Err(format!("expected Open source in clone, got {other:?}")),
        }
        match clone.target {
            WireEndpoint::Node(ref s) if s == "a" => {}
            other => return Err(format!("expected Node(\"a\") target in clone, got {other:?}")),
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §8  ArrangeMode / FlowDirection
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_08 {
    use super::*;

    #[test]
    fn should_distinguish_all_arrange_modes() -> Result<(), String> {
        let modes = [ArrangeMode::Grid, ArrangeMode::Horizontal, ArrangeMode::Vertical];

        for (i, a) in modes.iter().enumerate() {
            if a != a {
                return Err(format!("{a:?} did not equal itself"));
            }
            for (j, b) in modes.iter().enumerate() {
                if i != j && a == b {
                    return Err(format!("{a:?} incorrectly equalled {b:?}"));
                }
            }
        }
        Ok(())
    }

    #[test]
    fn should_distinguish_all_flow_directions() -> Result<(), String> {
        let dirs = [
            FlowDirection::LeftToRight,
            FlowDirection::TopToBottom,
            FlowDirection::RightToLeft,
            FlowDirection::BottomToTop,
        ];

        for (i, a) in dirs.iter().enumerate() {
            if a != a {
                return Err(format!("{a:?} did not equal itself"));
            }
            for (j, b) in dirs.iter().enumerate() {
                if i != j && a == b {
                    return Err(format!("{a:?} incorrectly equalled {b:?}"));
                }
            }
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §9  EventHandler
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_09 {
    use super::*;

    #[test]
    fn should_allow_empty_event_handler_body() -> Result<(), String> {
        let h = EventHandler {
            node: "a".into(),
            event: "receive".into(),
            params: vec![],
            body: vec![],
        };
        if !h.body.is_empty() {
            return Err(format!("expected empty body, got {} effects", h.body.len()));
        }
        Ok(())
    }

    #[test]
    fn should_clone_event_handler_independently_of_original() -> Result<(), String> {
        let original = EventHandler {
            node: "a".into(),
            event: "receive".into(),
            params: vec!["v".into()],
            body: vec![],
        };
        let mut clone = original.clone();
        clone.node = "b".into();
        clone.params.push("w".into());

        if original.node != "a" {
            return Err(format!(
                "original.node changed by mutation of clone: expected \"a\", got {:?}",
                original.node
            ));
        }
        if original.params.len() != 1 {
            return Err(format!(
                "original.params.len() changed by mutation of clone: expected 1, got {}",
                original.params.len()
            ));
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §10  Effects / Duration / RerouteDir
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_10 {
    use super::*;

    #[test]
    fn should_distinguish_reroute_directions() -> Result<(), String> {
        if RerouteDir::To == RerouteDir::From {
            return Err("RerouteDir::To incorrectly equalled RerouteDir::From".into());
        }
        if RerouteDir::To != RerouteDir::To {
            return Err("RerouteDir::To did not equal itself".into());
        }
        if RerouteDir::From != RerouteDir::From {
            return Err("RerouteDir::From did not equal itself".into());
        }
        Ok(())
    }

    #[test]
    fn should_clone_duration_preserving_value_and_unit() -> Result<(), String> {
        let original = Duration { value: 250, unit: DurationUnit::Ms };
        let clone = original.clone();

        if clone.value != 250 {
            return Err(format!("expected value 250, got {}", clone.value));
        }
        match clone.unit {
            DurationUnit::Ms => Ok(()),
            other => Err(format!("expected DurationUnit::Ms, got {other:?}")),
        }
    }

    #[test]
    fn should_clone_reroute_effect_independently_of_original() -> Result<(), String> {
        let original = Effect::Reroute(RerouteEffect {
            wire: "w1".into(),
            direction: RerouteDir::To,
            node: "dest".into(),
        });
        let mut clone = original.clone();

        // Mutate the clone's inner fields
        if let Effect::Reroute(ref mut r) = clone {
            r.wire = "w2".into();
        }

        // Original must be unchanged
        match &original {
            Effect::Reroute(r) if r.wire == "w1" => Ok(()),
            Effect::Reroute(r) => Err(format!(
                "original wire changed by mutation of clone: expected \"w1\", got {:?}",
                r.wire
            )),
            other => Err(format!("expected Reroute effect, got {other:?}")),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §11  Expr / BinOp
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_11 {
    use super::*;

    #[test]
    fn should_distinguish_all_binop_variants() -> Result<(), String> {
        let ops = [
            BinOp::Or,   BinOp::Xor,   BinOp::And,
            BinOp::Add,  BinOp::Sub,
            BinOp::Shl,  BinOp::ShrU,  BinOp::ShrS,
            BinOp::RotrU, BinOp::RotrS, BinOp::RotlU, BinOp::RotlS,
        ];

        for (i, a) in ops.iter().enumerate() {
            if a != a {
                return Err(format!("{a:?} did not equal itself"));
            }
            for (j, b) in ops.iter().enumerate() {
                if i != j && a == b {
                    return Err(format!("{a:?} incorrectly equalled {b:?}"));
                }
            }
        }
        Ok(())
    }

    #[test]
    fn should_build_nested_binop_expression() -> Result<(), String> {
        // Build: (a xor b) or c
        let inner = Expr::BinOp {
            op: BinOp::Xor,
            lhs: Box::new(Expr::Ident("a".into())),
            rhs: Box::new(Expr::Ident("b".into())),
        };
        let outer = Expr::BinOp {
            op: BinOp::Or,
            lhs: Box::new(inner),
            rhs: Box::new(Expr::Ident("c".into())),
        };

        match outer {
            Expr::BinOp { op: BinOp::Or, lhs, .. } => match *lhs {
                Expr::BinOp { op: BinOp::Xor, .. } => Ok(()),
                other => Err(format!("expected Xor inner node, got {other:?}")),
            },
            other => Err(format!("expected Or outer node, got {other:?}")),
        }
    }

    #[test]
    fn should_clone_expr_tree_independently_of_original() -> Result<(), String> {
        let original = Expr::BinOp {
            op: BinOp::Xor,
            lhs: Box::new(Expr::Ident("a".into())),
            rhs: Box::new(Expr::Ident("b".into())),
        };
        let clone = original.clone();

        // Both should independently be BinOp(Xor)
        match clone {
            Expr::BinOp { op: BinOp::Xor, .. } => {}
            other => return Err(format!("clone had wrong variant: expected BinOp(Xor), got {other:?}")),
        }

        // The original should still be intact
        match original {
            Expr::BinOp { op: BinOp::Xor, .. } => Ok(()),
            other => Err(format!("original changed after clone: expected BinOp(Xor), got {other:?}")),
        }
    }

    #[test]
    fn should_build_deeply_nested_expr_without_stack_overflow() -> Result<(), String> {
        // Construct a chain of 1 000 BinOp nodes to verify no overflow during
        // construction or the subsequent match and drop.
        let mut expr = Expr::Integer(0);
        for _ in 0..1_000 {
            expr = Expr::BinOp {
                op: BinOp::Xor,
                lhs: Box::new(expr),
                rhs: Box::new(Expr::Integer(0)),
            };
        }

        match expr {
            Expr::BinOp { .. } => Ok(()),
            other => Err(format!("expected BinOp at root after deep construction, got {other:?}")),
        }
    }
}
