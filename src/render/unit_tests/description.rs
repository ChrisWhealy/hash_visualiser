use super::{check, description_html, parse_and_build};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Node descriptions: a node's markdown `description` is rendered to HTML for the docs panel.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_render_node_description_markdown_to_html() -> Result<(), String> {
    let g = parse_and_build(
        "
        node c : operation {
            symbol: \"ThetaC\",
            description: \"\"\"
# Theta-C

XOR the five lanes via `theta_c`.

```rust
fn theta_c() {}
```
\"\"\"
        }
    ",
    );

    let html = description_html(&g.nodes["c"]).ok_or("expected a rendered description")?;
    check(html.contains("<h1>"), "heading should render to <h1>")?;
    check(
        html.contains("<code>theta_c</code>"),
        "inline code should render",
    )?;
    check(
        html.contains("<pre><code"),
        "fenced block should render to <pre><code>",
    )
}

#[test]
fn should_have_no_description_html_when_absent() -> Result<(), String> {
    let g = parse_and_build("node plain : register { format: hex8 }");
    check(
        description_html(&g.nodes["plain"]).is_none(),
        "a node without a description should produce no HTML",
    )
}
