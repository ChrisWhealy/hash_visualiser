/**
 * Tree-sitter grammar for the Hash Visualiser (.hv) DSL.
 *
 * Mirrors hv_grammar.ebnf. Property keys (label, format, source, compute, …) and format/word_size names are left as
 * plain identifiers rather than keywords, so they highlight as properties; only structural words, operators, node
 * kinds and flow/arrange/layout modes are reserved.
 */
module.exports = grammar({
  name: 'hv',

  word: $ => $.identifier,

  extras: $ => [/\s/, $.comment],

  rules: {
    source_file: $ => repeat($._top_item),

    comment: _ => token(seq('//', /[^\n]*/)),

    _top_item: $ => choice(
      $.context_block,
      $.function_definition,
      $.hash_block,
      $.node_declaration,
      $.wire_declaration,
      $.group_declaration,
      $.layout_declaration,
      $.import_declaration,
      $.data_declaration,
      $.event_handler,
    ),

    // - - - blocks - - -
    hash_block: $ => seq('hash', field('name', $.identifier), '{', repeat($._hash_item), '}'),
    _hash_item: $ => choice(
      $.context_block,
      $.function_definition,
      $.node_declaration,
      $.wire_declaration,
      $.group_declaration,
      $.layout_declaration,
      $.data_declaration,
      $.event_handler,
    ),

    context_block: $ => seq('context', '{', repeat($._context_item), '}'),
    _context_item: $ => choice($.function_definition, $.context_setting),
    context_setting: $ => seq(field('key', $.identifier), ':', field('value', $.integer)),

    // - - - functions - - -
    function_definition: $ => seq(
      'fn',
      field('name', $.identifier),
      '(', optional($.typed_parameters), ')',
      optional(seq('->', field('return_type', $.type))),
      '=',
      field('body', $._expr),
    ),
    typed_parameters: $ => seq($.parameter, repeat(seq(',', $.parameter)), optional(',')),
    parameter: $ => seq(field('name', $.identifier), ':', field('type', $.type)),

    type: $ => choice($.primitive_type, $.array_type),
    primitive_type: _ => choice('u8', 'u16', 'u32', 'u64'),
    array_type: $ => seq('[', $.type, ';', $.integer, ']'),

    // - - - nodes - - -
    node_declaration: $ => seq(
      'node',
      field('name', $.identifier),
      ':',
      field('kind', $.node_kind),
      optional($.property_block),
    ),
    node_kind: $ => choice('register', 'operation', 'constant', 'button', $.identifier),
    property_block: $ => seq('{', optional(seq($.property, repeat(seq(',', $.property)), optional(','))), '}'),
    property: $ => seq(field('key', $.identifier), ':', field('value', $._prop_value)),
    _prop_value: $ => choice($.string, $.triple_string, $._expr),

    // - - - data - - -
    data_declaration: $ => seq('data', field('name', $.identifier), '=', field('value', $._expr)),

    // - - - wires - - -
    wire_declaration: $ => seq(
      'wire',
      optional(seq(field('wire_name', $.identifier), ':')),
      field('source', $.wire_endpoint),
      '->',
      field('target', $.wire_endpoint),
    ),
    wire_endpoint: $ => choice($.identifier, '?'),

    // - - - layout & groups - - -
    layout_declaration: $ => seq('layout', ':', field('direction', $.flow_direction)),
    flow_direction: _ => choice('left_to_right', 'top_to_bottom', 'right_to_left', 'bottom_to_top'),

    // Bring another file's functions into scope; the string is the imported file's path (relative to the importer).
    import_declaration: $ => seq('import', field('path', $.string)),

    group_declaration: $ => seq('group', field('name', $.identifier), '{', repeat($._group_item), '}'),
    _group_item: $ => choice($.contains_declaration, $.arrange_declaration),
    contains_declaration: $ => seq('contains', ':', '[', $.identifier, repeat(seq(',', $.identifier)), optional(','), ']'),
    arrange_declaration: $ => seq('arrange', ':', field('mode', $.arrange_mode)),
    arrange_mode: _ => choice('grid', 'horizontal', 'vertical'),

    // - - - event handlers - - -
    event_handler: $ => seq(
      field('node', $.identifier),
      'on',
      field('event', $.identifier),
      '(', optional($.parameters), ')',
      '{', repeat($._effect), '}',
    ),
    parameters: $ => seq($.identifier, repeat(seq(',', $.identifier)), optional(',')),

    _effect: $ => choice($.set_effect, $.let_binding, $.animate_effect, $.emit_effect, $.reroute_effect),
    set_effect: $ => seq('set', choice($.prop_assignment, $.var_assignment, $.identifier)),
    prop_assignment: $ => seq(field('key', $.identifier), ':', field('value', $._expr)),
    var_assignment: $ => seq(field('name', $.identifier), '=', field('value', $._expr)),
    let_binding: $ => seq('let', field('name', $.identifier), '=', field('value', $._expr)),
    animate_effect: $ => seq('animate', choice($.fill_pulse, $.prop_transition)),
    fill_pulse: $ => seq(field('prop', $.identifier), ':', $.identifier, $.string, 'for', $.duration),
    prop_transition: $ => seq(field('prop', $.identifier), 'from', $._expr, 'to', $._expr, 'over', $.duration),
    emit_effect: $ => seq('emit', field('event', $.identifier), '(', optional($.arguments), ')', optional($.emit_target)),
    emit_target: $ => choice(seq('->', choice('all', $.identifier)), seq('via', $.identifier)),
    reroute_effect: $ => seq('reroute', $.identifier, choice('to', 'from'), $.identifier),

    // - - - expressions - - -
    _expr: $ => choice(
      $.binary_expression,
      $.unary_expression,
      $.index_expression,
      $._primary,
    ),

    binary_expression: $ => choice(
      prec.left(1, seq(field('left', $._expr), field('operator', 'or'), field('right', $._expr))),
      prec.left(2, seq(field('left', $._expr), field('operator', 'xor'), field('right', $._expr))),
      prec.left(3, seq(field('left', $._expr), field('operator', 'and'), field('right', $._expr))),
      prec.left(4, seq(field('left', $._expr), field('operator', choice('+', '-')), field('right', $._expr))),
      prec.left(5, seq(field('left', $._expr), field('operator', choice('shl', 'shr_u', 'shr_s')), field('right', $._expr))),
      prec.left(6, seq(field('left', $._expr), field('operator', choice('rotr_u', 'rotr_s', 'rotl_u', 'rotl_s')), field('right', $._expr))),
    ),

    unary_expression: $ => prec(7, seq(field('operator', 'not'), $._expr)),

    index_expression: $ => prec.left(8, seq(field('base', $._expr), '[', field('index', $._expr), ']')),

    _primary: $ => choice(
      $.integer,
      $.hex_literal,
      $.duration,
      $.call_expression,
      $.identifier,
      $.array_literal,
      $.comprehension,
      $.reduction,
      $.parenthesized_expression,
    ),

    call_expression: $ => seq(field('function', $.identifier), '(', optional($.arguments), ')'),
    arguments: $ => seq($._expr, repeat(seq(',', $._expr))),

    array_literal: $ => seq('[', optional(seq($._expr, repeat(seq(',', $._expr)), optional(','))), ']'),

    comprehension: $ => seq(
      '[', 'for', field('variable', $.identifier), 'in',
      field('start', $.integer), '..', field('end', $.integer),
      '=>', field('body', $._expr), ']',
    ),

    reduction: $ => seq('reduce', field('operator', choice('or', 'xor', 'and', '+')), 'over', field('array', $._expr)),

    parenthesized_expression: $ => seq('(', $._expr, ')'),

    // - - - literals - - -
    identifier: _ => /[A-Za-z_][A-Za-z0-9_]*/,
    integer: _ => token(/[0-9]+/),
    hex_literal: _ => token(seq('0x', /[0-9a-fA-F]+/)),
    duration: _ => token(seq(/[0-9]+/, choice('ms', 's'))),
    string: _ => token(seq('"', /[^"\n]*/, '"')),
    triple_string: _ => token(seq('"""', /([^"]|"[^"]|""[^"])*/, '"""')),
  },
});
