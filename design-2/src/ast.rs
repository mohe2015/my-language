pub enum AST {
    Integer(u64),
    Double(f64),
    Add(Vec<AST>),
    Multiply(Vec<AST>)
}