# Hash Visualizer

This is a work in progress.

The objective here is to develop a DSL-based tool for visualizing how data flows throught various hash algorithms such as SHA2 or SHA3.

## PoC ToDo List

- [x] Define formal grammar for the DSL
- [x] Write DSL lexer and parser
- [x] Create AST from the parser's token stream
- [ ] Perform semantic graph validation
- [ ] Construct directed graph
- [ ] Perform topological sort (Kahn's algorithm, layered output)
- [ ] Build layout engine that maps each topo-sort layer to a screen position
