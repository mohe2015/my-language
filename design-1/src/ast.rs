#[derive(Debug, PartialEq, Clone)]
pub struct Node {
    slice: String,
    pub inner: NodeInner,
}

#[derive(Debug, PartialEq, Clone)]
pub enum NodeInner {
    List(Vec<Node>),
    Symbol(String),
}

impl<'a> TryFrom<&'a Node> for &'a Vec<Node> {
    type Error = ();

    fn try_from(value: &'a Node) -> Result<Self, Self::Error> {
        match &value.inner {
            NodeInner::List(nodes) => Ok(nodes),
            NodeInner::Symbol(_) => panic!(),
        }
    }
}

impl<'a> TryFrom<&'a Node> for &'a str {
    type Error = ();

    fn try_from(value: &'a Node) -> Result<Self, Self::Error> {
        match &value.inner {
            NodeInner::List(nodes) => panic!(),
            NodeInner::Symbol(symbol) => Ok(symbol),
        }
    }
}

pub fn parse_toplevel(mut input: &str) -> Vec<Node> {
    let mut elems = Vec::new();
    while !input.trim_ascii_start().is_empty() {
        let elem;
        (input, elem) = parse(input);
        elems.push(elem);
    }
    elems
}

/// Returns unparsed remainder and node
pub fn parse(input: &str) -> (&str, Node) {
    let mut input = input.trim_ascii_start();
    if input.starts_with("(") {
        let list_start = input;
        input = &input[1..];
        // parse list
        let mut elems = Vec::new();
        loop {
            input = input.trim_ascii_start();
            if input.is_empty() {
                panic!("unclosed list at eof");
            }
            if input.starts_with(")") {
                break;
            }
            let elem;
            (input, elem) = parse(input);
            elems.push(elem);
        }
        input = &input[1..];
        (input, Node {
            inner: NodeInner::List(elems),
            slice: list_start[0..input.as_ptr() as usize - list_start.as_ptr() as usize].to_owned(),
        })
    } else {
        // parse one symbol but dont eat closing parenc
        let idx = input
            .find(|c| char::is_ascii_whitespace(&c) || c == ')')
            .unwrap_or(input.len());
        let (symbol, rest) = input.split_at_checked(idx).unwrap();
        if symbol.is_empty() {
            panic!("symbol must not be empty")
        }
        (rest, Node {
            inner: NodeInner::Symbol(symbol.to_owned()),
            slice: symbol.to_owned(),
        })
    }
}
