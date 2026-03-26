//! Indentation-based parser for the markup format.
//!
//! Produces a `Vec<Node>` from a multi-line string. The grammar (informal):
//!
//! ```text
//! document     = { blank | comment | node_line }
//! node_line    = INDENT widget_type [ "." variant ] [ QUOTED_STRING ] { prop }
//! prop         = IDENT "=" value
//! value        = QUOTED_STRING | NUMBER | BOOL | COLOR | ARRAY | IDENT
//! ARRAY        = "[" value { "," value } "]"
//! ```

use std::collections::HashMap;

use crate::ast::{Node, Value, WidgetKind};

/// A parse error with location and message.
#[derive(Debug, Clone, thiserror::Error)]
#[error("line {line}: {message}")]
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl ParseError {
    fn new(line: usize, col: usize, msg: impl Into<String>) -> Self {
        Self {
            line,
            column: col,
            message: msg.into(),
        }
    }
}

/// Parse markup text into a list of root nodes.
pub fn parse(input: &str) -> Result<Vec<Node>, ParseError> {
    let mut root_nodes: Vec<Node> = Vec::new();
    // Stack: (indent_level, node). We'll build the tree by tracking parents.
    let mut stack: Vec<(usize, Node)> = Vec::new();

    for (line_idx, raw_line) in input.lines().enumerate() {
        let line_num = line_idx + 1;

        // Skip blank lines.
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Skip comments.
        if trimmed.starts_with("//") {
            continue;
        }

        // Measure indentation (count leading spaces; tabs = 4 spaces).
        let indent = raw_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(|c| if c == '\t' { 4 } else { 1 })
            .sum::<usize>();

        // Parse the content after indentation.
        let content = trimmed;
        let node = parse_line(content, line_num)?;

        // Pop nodes from stack that are at >= our indent level, attaching them
        // as children of their parent.
        while let Some(&(parent_indent, _)) = stack.last() {
            if parent_indent >= indent {
                let (_, child) = stack.pop().unwrap();
                if let Some((_, parent)) = stack.last_mut() {
                    parent.children.push(child);
                } else {
                    root_nodes.push(child);
                }
            } else {
                break;
            }
        }

        stack.push((indent, node));
    }

    // Flush remaining stack.
    while let Some((_, child)) = stack.pop() {
        if let Some((_, parent)) = stack.last_mut() {
            parent.children.push(child);
        } else {
            root_nodes.push(child);
        }
    }

    Ok(root_nodes)
}

/// Parse a single (trimmed) line into a `Node`.
fn parse_line(line: &str, line_num: usize) -> Result<Node, ParseError> {
    let mut cursor = Cursor::new(line, line_num);

    // 1. Widget type (with optional .variant).
    let type_name = cursor.read_ident()?;
    let (kind_str, variant) = if let Some((k, v)) = type_name.split_once('.') {
        (k, Some(v.to_string()))
    } else {
        (type_name.as_str(), None)
    };
    let kind = WidgetKind::from_name(kind_str);

    cursor.skip_spaces();

    // 2. Optional quoted text content.
    let text = if cursor.peek() == Some('"') {
        Some(cursor.read_quoted_string()?)
    } else {
        None
    };

    cursor.skip_spaces();

    // 3. Properties: key=value pairs.
    let mut props = HashMap::new();
    while !cursor.is_empty() {
        cursor.skip_spaces();
        if cursor.is_empty() {
            break;
        }

        // Inline comment ends the line.
        if cursor.rest().starts_with("//") {
            break;
        }

        let key = cursor.read_ident()?;
        if cursor.peek() != Some('=') {
            return Err(ParseError::new(
                line_num,
                cursor.pos,
                format!("expected '=' after property '{key}'"),
            ));
        }
        cursor.advance(); // skip '='
        let value = cursor.read_value()?;
        props.insert(key, value);
    }

    Ok(Node {
        kind,
        variant,
        text,
        props,
        children: Vec::new(),
        line: line_num,
    })
}

// ── Cursor ──────────────────────────────────────────────────────────────────

/// A simple cursor over a line for character-by-character parsing.
struct Cursor<'a> {
    src: &'a str,
    pos: usize,
    line: usize,
}

impl<'a> Cursor<'a> {
    fn new(src: &'a str, line: usize) -> Self {
        Self { src, pos: 0, line }
    }

    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn is_empty(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
        }
    }

    fn skip_spaces(&mut self) {
        while self.peek() == Some(' ') {
            self.advance();
        }
    }

    /// Read an identifier: `[a-zA-Z_][a-zA-Z0-9_-]*` (allows hyphens for
    /// widget names like `max-width`, `split-pane`).
    /// Also allows dots for `button.primary` when reading the widget type.
    fn read_ident(&mut self) -> Result<String, ParseError> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
                self.advance();
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(ParseError::new(
                self.line,
                self.pos,
                format!("expected identifier, got {:?}", self.peek().unwrap_or('\0')),
            ));
        }
        Ok(self.src[start..self.pos].to_string())
    }

    /// Read a `"quoted string"`, handling `\"` and `\\` escapes.
    fn read_quoted_string(&mut self) -> Result<String, ParseError> {
        if self.peek() != Some('"') {
            return Err(ParseError::new(self.line, self.pos, "expected '\"'"));
        }
        self.advance(); // skip opening "

        let mut result = String::new();
        loop {
            match self.peek() {
                None => return Err(ParseError::new(self.line, self.pos, "unterminated string")),
                Some('"') => {
                    self.advance();
                    return Ok(result);
                }
                Some('\\') => {
                    self.advance();
                    match self.peek() {
                        Some('"') => {
                            result.push('"');
                            self.advance();
                        }
                        Some('\\') => {
                            result.push('\\');
                            self.advance();
                        }
                        Some('n') => {
                            result.push('\n');
                            self.advance();
                        }
                        Some('t') => {
                            result.push('\t');
                            self.advance();
                        }
                        Some(c) => {
                            result.push('\\');
                            result.push(c);
                            self.advance();
                        }
                        None => {
                            return Err(ParseError::new(
                                self.line,
                                self.pos,
                                "unterminated escape",
                            ));
                        }
                    }
                }
                Some(c) => {
                    result.push(c);
                    self.advance();
                }
            }
        }
    }

    /// Read a value (right-hand side of `key=`).
    fn read_value(&mut self) -> Result<Value, ParseError> {
        match self.peek() {
            Some('"') => Ok(Value::String(self.read_quoted_string()?)),
            Some('#') => self.read_color(),
            Some('[') => self.read_array(),
            Some(c) if c == '-' || c.is_ascii_digit() => self.read_number(),
            Some(c) if c.is_alphabetic() || c == '_' => self.read_bool_or_ident(),
            Some(c) => Err(ParseError::new(
                self.line,
                self.pos,
                format!("unexpected character '{c}' in value"),
            )),
            None => Err(ParseError::new(self.line, self.pos, "expected value")),
        }
    }

    /// Read a hex color: `#rgb`, `#rrggbb`, or `#rrggbbaa`.
    fn read_color(&mut self) -> Result<Value, ParseError> {
        self.advance(); // skip '#'
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_hexdigit() {
                self.advance();
            } else {
                break;
            }
        }
        let hex = &self.src[start..self.pos];
        let expanded = match hex.len() {
            3 => {
                // #rgb → #rrggbb
                let mut s = String::with_capacity(6);
                for c in hex.chars() {
                    s.push(c);
                    s.push(c);
                }
                s
            }
            6 | 8 => hex.to_string(),
            _ => {
                return Err(ParseError::new(
                    self.line,
                    start - 1,
                    format!("invalid color: #{hex} (expected 3, 6, or 8 hex digits)"),
                ));
            }
        };
        let val = u32::from_str_radix(&expanded, 16).map_err(|_| {
            ParseError::new(self.line, start - 1, format!("invalid hex color: #{hex}"))
        })?;
        Ok(Value::Color(val))
    }

    /// Read a number (integer or float, possibly negative).
    fn read_number(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.advance();
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                self.advance();
            } else {
                break;
            }
        }
        let s = &self.src[start..self.pos];
        let n: f64 = s
            .parse()
            .map_err(|_| ParseError::new(self.line, start, format!("invalid number: {s}")))?;
        Ok(Value::Number(n))
    }

    /// Read `true`, `false`, or a bare identifier.
    fn read_bool_or_ident(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                self.advance();
            } else {
                break;
            }
        }
        let word = &self.src[start..self.pos];
        match word {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Ok(Value::Ident(word.to_string())),
        }
    }

    /// Read an array: `[value, value, ...]`.
    fn read_array(&mut self) -> Result<Value, ParseError> {
        self.advance(); // skip '['
        self.skip_spaces();

        let mut items = Vec::new();

        // Handle empty array.
        if self.peek() == Some(']') {
            self.advance();
            return Ok(Value::Array(items));
        }

        loop {
            self.skip_spaces();
            items.push(self.read_value()?);
            self.skip_spaces();

            match self.peek() {
                Some(',') => {
                    self.advance();
                    // Allow trailing comma before ']'.
                    self.skip_spaces();
                    if self.peek() == Some(']') {
                        self.advance();
                        return Ok(Value::Array(items));
                    }
                }
                Some(']') => {
                    self.advance();
                    return Ok(Value::Array(items));
                }
                _ => {
                    return Err(ParseError::new(
                        self.line,
                        self.pos,
                        "expected ',' or ']' in array",
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_label() {
        let nodes = parse(r#"label "Hello""#).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].kind, WidgetKind::Label);
        assert_eq!(nodes[0].text.as_deref(), Some("Hello"));
    }

    #[test]
    fn parse_widget_with_variant() {
        let nodes = parse(r#"button.primary "Save""#).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].kind, WidgetKind::Button);
        assert_eq!(nodes[0].variant.as_deref(), Some("primary"));
        assert_eq!(nodes[0].text.as_deref(), Some("Save"));
    }

    #[test]
    fn parse_properties() {
        let nodes = parse(r#"input placeholder="Enter name" rows=4"#).unwrap();
        assert_eq!(nodes[0].prop_str("placeholder"), Some("Enter name"));
        assert_eq!(nodes[0].prop_f64("rows"), Some(4.0));
    }

    #[test]
    fn parse_color_property() {
        let nodes = parse(r#"card bg=#1a1a2e"#).unwrap();
        assert_eq!(nodes[0].prop_color("bg"), Some(0x1a1a2e));
    }

    #[test]
    fn parse_short_color() {
        let nodes = parse(r#"card bg=#f00"#).unwrap();
        assert_eq!(nodes[0].prop_color("bg"), Some(0xff0000));
    }

    #[test]
    fn parse_array_property() {
        let nodes = parse(r#"select options=["Light", "Dark", "System"]"#).unwrap();
        let opts = nodes[0].prop_string_array("options").unwrap();
        assert_eq!(opts, vec!["Light", "Dark", "System"]);
    }

    #[test]
    fn parse_number_array() {
        let nodes = parse(r#"columns weights=[1, 2, 1]"#).unwrap();
        let w = nodes[0].prop_number_array("weights").unwrap();
        assert_eq!(w, vec![1.0, 2.0, 1.0]);
    }

    #[test]
    fn parse_bool_property() {
        let nodes = parse(r#"modal open=true"#).unwrap();
        assert_eq!(nodes[0].prop_bool("open"), Some(true));
    }

    #[test]
    fn parse_ident_property() {
        let nodes = parse(r#"row justify=between align=center"#).unwrap();
        assert_eq!(nodes[0].prop_str("justify"), Some("between"));
        assert_eq!(nodes[0].prop_str("align"), Some("center"));
    }

    #[test]
    fn parse_indentation_hierarchy() {
        let input = "\
page
  heading \"Title\"
  card
    label \"Inside card\"
    button \"Click\"
  label \"Outside card\"";

        let nodes = parse(input).unwrap();
        assert_eq!(nodes.len(), 1); // one root: page
        let page = &nodes[0];
        assert_eq!(page.children.len(), 3); // heading, card, label

        let card = &page.children[1];
        assert_eq!(card.kind, WidgetKind::Card);
        assert_eq!(card.children.len(), 2); // label, button
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let input = "\
heading \"A\"

heading \"B\"";
        let nodes = parse(input).unwrap();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn parse_comments_ignored() {
        let input = "\
// This is a comment
heading \"Title\"
// Another comment
label \"Text\"";
        let nodes = parse(input).unwrap();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn parse_inline_comment() {
        let nodes = parse(r#"button "Save" // primary action"#).unwrap();
        assert_eq!(nodes[0].text.as_deref(), Some("Save"));
        assert!(nodes[0].props.is_empty());
    }

    #[test]
    fn parse_aliases() {
        // Various aliases should resolve to the same kind.
        let cases = [
            ("btn \"X\"", WidgetKind::Button),
            ("h \"X\"", WidgetKind::Heading),
            ("p \"X\"", WidgetKind::Paragraph),
            ("hr", WidgetKind::Separator),
            ("col", WidgetKind::Column),
            ("cols weights=[1]", WidgetKind::Columns),
            ("scroll height=200", WidgetKind::Scrollable),
            ("dropdown options=[\"a\"]", WidgetKind::Select),
            ("switch \"Dark mode\"", WidgetKind::Toggle),
            ("dialog", WidgetKind::Modal),
            ("loading", WidgetKind::Spinner),
            ("tag \"v1\"", WidgetKind::Chip),
            ("stars", WidgetKind::Rating),
            ("img", WidgetKind::Image),
            ("steps", WidgetKind::Stepper),
            ("empty", WidgetKind::EmptyState),
            ("collapse", WidgetKind::Collapsing),
        ];
        for (input, expected) in cases {
            let nodes = parse(input).unwrap();
            assert_eq!(nodes[0].kind, expected, "alias failed for: {input}");
        }
    }

    #[test]
    fn parse_deep_nesting() {
        let input = "\
page
  card
    row
      col
        label \"Deep\"";
        let nodes = parse(input).unwrap();
        let page = &nodes[0];
        let card = &page.children[0];
        let row = &card.children[0];
        let col = &row.children[0];
        let label = &col.children[0];
        assert_eq!(label.text.as_deref(), Some("Deep"));
    }

    #[test]
    fn parse_multiple_roots() {
        let input = "\
heading \"One\"
heading \"Two\"
heading \"Three\"";
        let nodes = parse(input).unwrap();
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn parse_negative_number() {
        let nodes = parse(r#"space amount=-8"#).unwrap();
        assert_eq!(nodes[0].prop_f64("amount"), Some(-8.0));
    }

    #[test]
    fn parse_float_number() {
        let nodes = parse(r#"progress value=0.75"#).unwrap();
        assert_eq!(nodes[0].prop_f64("value"), Some(0.75));
    }

    #[test]
    fn parse_trailing_comma_in_array() {
        let nodes = parse(r#"tabs labels=["A", "B",]"#).unwrap();
        let arr = nodes[0].prop_string_array("labels").unwrap();
        assert_eq!(arr, vec!["A", "B"]);
    }

    #[test]
    fn parse_empty_array() {
        let nodes = parse(r#"select options=[]"#).unwrap();
        let arr = nodes[0].prop_string_array("options").unwrap();
        assert!(arr.is_empty());
    }

    #[test]
    fn parse_string_escapes() {
        let nodes = parse(r#"label "say \"hello\"""#).unwrap();
        assert_eq!(nodes[0].text.as_deref(), Some(r#"say "hello""#));
    }

    #[test]
    fn parse_unknown_widget_is_custom() {
        let nodes = parse(r#"my-widget "custom""#).unwrap();
        assert_eq!(nodes[0].kind, WidgetKind::Custom("my-widget".to_string()));
    }

    #[test]
    fn error_unterminated_string() {
        let result = parse(r#"label "oops"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("unterminated"));
    }

    #[test]
    fn error_missing_equals() {
        let result = parse(r#"input placeholder"hello""#);
        assert!(result.is_err());
    }

    #[test]
    fn parse_realistic_form() {
        let input = r#"
page padding=24
  heading "Account Settings" size=xl

  card
    field "Display Name" status=ok
      input placeholder="Enter your name"

    field "Email" status=error hint="Invalid email format"
      input placeholder="user@example.com"

    field "Role"
      select options=["Admin", "Editor", "Viewer"]

    separator

    row gap=16 justify=end
      button.secondary "Cancel"
      button.primary "Save Changes"
"#;
        let nodes = parse(input).unwrap();
        assert_eq!(nodes.len(), 1);
        let page = &nodes[0];
        assert_eq!(page.kind, WidgetKind::Page);
        assert_eq!(page.prop_f32("padding"), Some(24.0));
        assert_eq!(page.children.len(), 2); // heading, card

        let card = &page.children[1];
        assert_eq!(card.children.len(), 5); // 3 fields, separator, row

        let row = &card.children[4];
        assert_eq!(row.kind, WidgetKind::Row);
        assert_eq!(row.children.len(), 2); // cancel, save
        assert_eq!(row.children[1].variant.as_deref(), Some("primary"));
    }

    #[test]
    fn parse_sibling_ordering() {
        let input = "\
row
  label \"A\"
  label \"B\"
  label \"C\"";
        let nodes = parse(input).unwrap();
        let row = &nodes[0];
        assert_eq!(row.children.len(), 3);
        assert_eq!(row.children[0].text.as_deref(), Some("A"));
        assert_eq!(row.children[1].text.as_deref(), Some("B"));
        assert_eq!(row.children[2].text.as_deref(), Some("C"));
    }

    #[test]
    fn parse_tab_indentation() {
        let input = "page\n\theading \"Tab indented\"";
        let nodes = parse(input).unwrap();
        assert_eq!(nodes[0].children.len(), 1);
    }

    #[test]
    fn parse_color_with_alpha() {
        let nodes = parse(r#"card bg=#1a1a2eff"#).unwrap();
        assert_eq!(nodes[0].prop_color("bg"), Some(0x1a1a2eff));
    }

    // ── New widget type tests ───────────────────────────────────────

    #[test]
    fn parse_new_widget_aliases() {
        let cases = [
            ("container", WidgetKind::Container),
            ("box", WidgetKind::Container),
            ("clip", WidgetKind::Clip),
            ("labeled", WidgetKind::Labeled),
            ("rich-text", WidgetKind::RichText),
            ("richtext", WidgetKind::RichText),
            ("rich", WidgetKind::RichText),
            ("span \"text\"", WidgetKind::Span),
            ("drop-zone", WidgetKind::DropZone),
            ("dropzone", WidgetKind::DropZone),
            ("pill \"Active\"", WidgetKind::StatusPill),
            ("status-pill \"Active\"", WidgetKind::StatusPill),
            ("pagination total-pages=10", WidgetKind::Pagination),
            ("pages total-pages=10", WidgetKind::Pagination),
            ("table-column \"Name\"", WidgetKind::TableColumn),
            ("th \"Name\"", WidgetKind::TableColumn),
            ("tree-node \"Item\"", WidgetKind::TreeNode),
            ("virtual-scroll height=400", WidgetKind::VirtualScroll),
            ("vscroll height=400", WidgetKind::VirtualScroll),
            ("menu-bar", WidgetKind::MenuBar),
            ("menubar", WidgetKind::MenuBar),
            ("menu \"File\"", WidgetKind::Menu),
            ("menu-item \"Save\"", WidgetKind::MenuItem),
        ];
        for (input, expected) in cases {
            let nodes = parse(input).unwrap();
            assert_eq!(nodes[0].kind, expected, "alias failed for: {input}");
        }
    }

    #[test]
    fn parse_rich_text_with_spans() {
        let input = "\
rich-text
  span \"Normal \"
  span \"bold\" weight=bold
  span \"colored\" color=#ff0000
  span \"sized\" size=18";
        let nodes = parse(input).unwrap();
        assert_eq!(nodes[0].kind, WidgetKind::RichText);
        assert_eq!(nodes[0].children.len(), 4);

        let bold = &nodes[0].children[1];
        assert_eq!(bold.kind, WidgetKind::Span);
        assert_eq!(bold.text.as_deref(), Some("bold"));
        assert_eq!(bold.prop_str("weight"), Some("bold"));

        let colored = &nodes[0].children[2];
        assert_eq!(colored.prop_color("color"), Some(0xff0000));
    }

    #[test]
    fn parse_table_with_columns() {
        let input = "\
table rows=100 visible=20
  th \"Name\" width=200
  th \"Email\" width=\"2fr\"
  th \"Role\" width=auto sortable=false";
        let nodes = parse(input).unwrap();
        assert_eq!(nodes[0].kind, WidgetKind::Table);
        assert_eq!(nodes[0].prop_f64("rows"), Some(100.0));
        assert_eq!(nodes[0].children.len(), 3);

        let name_col = &nodes[0].children[0];
        assert_eq!(name_col.kind, WidgetKind::TableColumn);
        assert_eq!(name_col.text.as_deref(), Some("Name"));
        assert_eq!(name_col.prop_f32("width"), Some(200.0));

        let email_col = &nodes[0].children[1];
        assert_eq!(email_col.prop_str("width"), Some("2fr"));

        let role_col = &nodes[0].children[2];
        assert_eq!(role_col.prop_str("width"), Some("auto"));
        assert_eq!(role_col.prop_bool("sortable"), Some(false));
    }

    #[test]
    fn parse_menu_bar() {
        let input = "\
menu-bar
  menu \"File\"
    menu-item \"New\" action=new
    menu-item \"Open\" action=open
    menu-item \"Save\" action=save
  menu \"Edit\"
    menu-item \"Undo\" action=undo";
        let nodes = parse(input).unwrap();
        assert_eq!(nodes[0].kind, WidgetKind::MenuBar);
        assert_eq!(nodes[0].children.len(), 2);

        let file_menu = &nodes[0].children[0];
        assert_eq!(file_menu.kind, WidgetKind::Menu);
        assert_eq!(file_menu.text.as_deref(), Some("File"));
        assert_eq!(file_menu.children.len(), 3);

        let save = &file_menu.children[2];
        assert_eq!(save.kind, WidgetKind::MenuItem);
        assert_eq!(save.text.as_deref(), Some("Save"));
        assert_eq!(save.prop_str("action"), Some("save"));
    }

    #[test]
    fn parse_virtual_scroll() {
        let input = r#"virtual-scroll item-height=32 height=400 count=10000"#;
        let nodes = parse(input).unwrap();
        assert_eq!(nodes[0].kind, WidgetKind::VirtualScroll);
        assert_eq!(nodes[0].prop_f32("item-height"), Some(32.0));
        assert_eq!(nodes[0].prop_f32("height"), Some(400.0));
        assert_eq!(nodes[0].prop_f64("count"), Some(10000.0));
    }

    #[test]
    fn parse_flex_with_child_props() {
        let input = "\
flex.row gap=16 align=center justify=between wrap=wrap
  container grow=1 shrink=0
    label \"Flexible\"
  container basis=200
    button \"Fixed\"";
        let nodes = parse(input).unwrap();
        let flex = &nodes[0];
        assert_eq!(flex.kind, WidgetKind::Flex);
        assert_eq!(flex.variant.as_deref(), Some("row"));
        assert_eq!(flex.prop_f32("gap"), Some(16.0));
        assert_eq!(flex.prop_str("align"), Some("center"));
        assert_eq!(flex.prop_str("justify"), Some("between"));
        assert_eq!(flex.prop_str("wrap"), Some("wrap"));

        let child0 = &flex.children[0];
        assert_eq!(child0.prop_f32("grow"), Some(1.0));
        assert_eq!(child0.prop_f32("shrink"), Some(0.0));

        let child1 = &flex.children[1];
        assert_eq!(child1.prop_f32("basis"), Some(200.0));
    }

    #[test]
    fn parse_grid_with_cell_placement() {
        let input = "\
grid cols=\"1fr 200 auto\" rows=\"auto 1fr\" gap=16
  container col=0 row=0 col-span=2
    heading \"Header spans two columns\"
  container col=0 row=1
    label \"Left\"
  container col=1 row=1
    label \"Right\"";
        let nodes = parse(input).unwrap();
        assert_eq!(nodes[0].kind, WidgetKind::Grid);
        assert_eq!(nodes[0].prop_str("cols"), Some("1fr 200 auto"));
        assert_eq!(nodes[0].prop_f32("gap"), Some(16.0));

        let header = &nodes[0].children[0];
        assert_eq!(header.prop_f64("col"), Some(0.0));
        assert_eq!(header.prop_f64("row"), Some(0.0));
        assert_eq!(header.prop_f64("col-span"), Some(2.0));
    }

    #[test]
    fn parse_style_properties() {
        let input = "\
style bg=#1a1a2e fg=#ffffff radius=8 opacity=0.9 border-width=1 border-color=#333
  card
    label \"Styled\"";
        let nodes = parse(input).unwrap();
        assert_eq!(nodes[0].kind, WidgetKind::Style);
        assert_eq!(nodes[0].prop_color("bg"), Some(0x1a1a2e));
        assert_eq!(nodes[0].prop_color("fg"), Some(0xffffff));
        assert_eq!(nodes[0].prop_f32("radius"), Some(8.0));
        assert_eq!(nodes[0].prop_f64("opacity"), Some(0.9));
        assert_eq!(nodes[0].prop_f32("border-width"), Some(1.0));
    }

    #[test]
    fn parse_spacing_as_array() {
        // Uniform
        let n = parse(r#"padding amount=16"#).unwrap();
        assert_eq!(n[0].prop_f32("amount"), Some(16.0));

        // Per-side as array
        let n = parse(r#"style padding=[8, 16, 8, 16]"#).unwrap();
        let p = n[0].prop_number_array("padding").unwrap();
        assert_eq!(p, vec![8.0, 16.0, 8.0, 16.0]);
    }

    #[test]
    fn parse_per_corner_radius() {
        let n = parse(r#"container radius=[8, 8, 0, 0]"#).unwrap();
        let r = n[0].prop_number_array("radius").unwrap();
        assert_eq!(r, vec![8.0, 8.0, 0.0, 0.0]);
    }

    #[test]
    fn parse_action_and_bind() {
        let n = parse(r#"button.primary "Save" action=save-settings"#).unwrap();
        assert_eq!(n[0].prop_str("action"), Some("save-settings"));

        let n = parse(r#"input bind=username placeholder="Name""#).unwrap();
        assert_eq!(n[0].prop_str("bind"), Some("username"));
    }

    #[test]
    fn parse_tooltip_and_a11y() {
        let n =
            parse(r#"button "Help" tooltip="Get help" a11y-label="Help button" a11y-role=button"#)
                .unwrap();
        assert_eq!(n[0].prop_str("tooltip"), Some("Get help"));
        assert_eq!(n[0].prop_str("a11y-label"), Some("Help button"));
        assert_eq!(n[0].prop_str("a11y-role"), Some("button"));
    }

    #[test]
    fn parse_text_style_properties() {
        let n = parse(
            r#"label "HELLO" text-align=center text-decoration=underline text-transform=uppercase font-size=18"#,
        )
        .unwrap();
        assert_eq!(n[0].prop_str("text-align"), Some("center"));
        assert_eq!(n[0].prop_str("text-decoration"), Some("underline"));
        assert_eq!(n[0].prop_str("text-transform"), Some("uppercase"));
        assert_eq!(n[0].prop_f32("font-size"), Some(18.0));
    }

    #[test]
    fn parse_gradient_properties() {
        let n = parse(r#"card gradient=linear gradient-to=#ff0000 gradient-angle=45"#).unwrap();
        assert_eq!(n[0].prop_str("gradient"), Some("linear"));
        assert_eq!(n[0].prop_color("gradient-to"), Some(0xff0000));
        assert_eq!(n[0].prop_f32("gradient-angle"), Some(45.0));
    }

    #[test]
    fn parse_elevation_presets_and_custom() {
        let n = parse(r#"card elevation=high"#).unwrap();
        assert_eq!(n[0].prop_str("elevation"), Some("high"));

        let n =
            parse(r#"card shadow-blur=10 shadow-dx=2 shadow-dy=4 shadow-color=#000000"#).unwrap();
        assert_eq!(n[0].prop_f32("shadow-blur"), Some(10.0));
        assert_eq!(n[0].prop_f32("shadow-dx"), Some(2.0));
    }

    #[test]
    fn parse_transform_properties() {
        let n =
            parse(r#"container translate-x=10 translate-y=20 scale-x=1.5 scale-y=1.5"#).unwrap();
        assert_eq!(n[0].prop_f32("translate-x"), Some(10.0));
        assert_eq!(n[0].prop_f32("scale-y"), Some(1.5));
    }

    #[test]
    fn parse_constraint_properties() {
        let n = parse(
            r#"container min-width=100 max-width=600 min-height=32 max-height=400 width=300"#,
        )
        .unwrap();
        assert_eq!(n[0].prop_f32("min-width"), Some(100.0));
        assert_eq!(n[0].prop_f32("max-width"), Some(600.0));
        assert_eq!(n[0].prop_f32("width"), Some(300.0));
    }

    #[test]
    fn parse_full_dashboard_example() {
        let input = r#"
page padding=24 max-width=1200

  // Header
  row justify=between align=center
    heading "Dashboard"
    row gap=8
      button.ghost "Settings" action=settings
      avatar "JD" size=32

  // Stats row
  columns weights=[1, 1, 1, 1] gap=16
    card
      label.muted "Revenue"
      heading "$42,500"
      pill.success "↑ 12%"
    card
      label.muted "Users"
      heading "1,234"
      pill.warning "↑ 3%"
    card
      label.muted "Orders"
      heading "356"
      pill.error "↓ 5%"
    card
      label.muted "Conversion"
      heading "4.2%"

  // Main content
  columns weights=[2, 1] gap=16

    // Table
    card
      heading "Recent Orders"
      table rows=50 visible=10
        th "Order" width=auto
        th "Customer" width="1fr"
        th "Amount" width=100
        th "Status" width=100

    // Sidebar
    col gap=16
      card
        heading "Activity"
        virtual-scroll item-height=48 height=300 count=100

      card
        heading "Quick Actions"
        col gap=8
          button "New Order" action=new-order
          button.secondary "Export Data" action=export
          button.ghost "View Reports" action=reports
"#;
        let nodes = parse(input).unwrap();
        assert_eq!(nodes.len(), 1);
        let page = &nodes[0];
        assert_eq!(page.kind, WidgetKind::Page);
        // header row, stats columns, content columns
        assert_eq!(page.children.len(), 3);

        // Stats columns
        let stats = &page.children[1];
        assert_eq!(stats.kind, WidgetKind::Columns);
        assert_eq!(stats.children.len(), 4);

        // Content columns
        let content = &page.children[2];
        assert_eq!(content.kind, WidgetKind::Columns);
        assert_eq!(content.children.len(), 2);

        // Table inside first card of content
        let table_card = &content.children[0];
        let table = &table_card.children[1];
        assert_eq!(table.kind, WidgetKind::Table);
        assert_eq!(table.children.len(), 4); // 4 column definitions
    }
}
