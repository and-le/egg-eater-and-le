mod infra;
success_tests! {
    {
        name: simple_examples_1,
        file: "simple_examples_1.snek",
        expected: "10\n20\n[10, 20]\n30\n40\nnil\n[30, 40, nil]"
    },
    {
        name: simple_examples_2,
        file: "simple_examples_2.snek",
        expected: "[[10, 20, nil], [30, 40, nil]]"
    },
    {
        name: points,
        file: "points.snek",
        expected: "[5, 10]\n[30, 60]\n[35, 70]"
    },
    {
        name: bst,
        file: "bst.snek",
        expected: "[50, [25, [0, nil, nil], nil], [75, nil, [100, nil, nil]]]\ntrue\ntrue\ntrue\nfalse\nfalse"
    },
    {
        name: simple_examples_3,
        file: "simple_examples_3.snek",
        expected: "[30, 40, 50]\n[60, 70, 80]"
    },
    {
        name: equal,
        file: "equal.snek",
        expected: "true\nfalse\ntrue\nfalse\ntrue\nfalse\ntrue\ntrue\nfalse"
    },
    {
        name: cyclic_print1,
        file: "cyclic-print1.snek",
        expected: "[10, 20, 30]\n[10, [...], 30]"
    },
    {
        name: cyclic_print2,
        file: "cyclic-print2.snek",
        expected: "[10, 20, 30]\n[40, 50, 60]\n[10, [40, [...], 60], 30]\n[40, [...], 60]"
    },
    {
        name: cyclic_print3,
        file: "cyclic-print3.snek",
        expected: "[10, [20, [30, [40, [50, nil]]]]]\n[10, [20, [30, [40, [...]]]]]"
    },
    {
        name: cyclic_equal1,
        file: "cyclic-equal1.snek",
        expected: "true"
    },
    {
        name: cyclic_equal2,
        file: "cyclic-equal2.snek",
        expected: "false"
    },
    {
        name: cyclic_equal3,
        file: "cyclic-equal3.snek",
        expected: "true"
    },

    {
        name: vec_len,
        file: "vec-len.snek",
        expected: "4"
    },
    {
        name: vec_get_1,
        file: "vec-get.snek",
        input: "0",
        expected: "10"
    },
    {
        name: vec_get_2,
        file: "vec-get.snek",
        input: "3",
        expected: "40"
    },
    {
        name: vec_set_1,
        file: "vec-set.snek",
        input: "0",
        expected: "[231, 20, 30, 40]"
    },
    {
        name: vec_set_2,
        file: "vec-set.snek",
        input: "3",
        expected: "[10, 20, 30, 231]"
    },
    {
        name: isvec1,
        file: "isvec1.snek",
        expected: "false"
    },
    {
        name: isvec2,
        file: "isvec2.snek",
        expected: "false"
    },
    {
        name: isvec3,
        file: "isvec3.snek",
        expected: "true"
    },
    {
        name: isvec4,
        file: "isvec4.snek",
        expected: "true"
    },
    {
        name: make_vec_1,
        file: "make_vec_1.snek",
        expected: "5\n231\n231\n231\n231\n231\n[231, 231, 231, 231, 231]"
    },

}

runtime_error_tests! {
    {
        name: error_bounds_1,
        file: "error-bounds.snek",
        input: "-1",
        expected: "index out of bounds"
    },
    {
        name: error_bounds_2,
        file: "error-bounds.snek",
        input: "5",
        expected: "index out of bounds"
    },
    {
        name: error_tag,
        file: "error-tag.snek",
        expected: "invalid vector address"
    },
    {
        name: error3,
        file: "error3.snek",
        expected: "invalid vector offset"
    },
    {
        name: error_vec_get_nil,
        file: "error-vec-get-nil.snek",
        expected: "invalid vector address"
    },

    {
        name: error_vec_get_1,
        file: "vec-get.snek",
        input: "-1",
        expected: "index out of bounds"
    },
    {
        name: error_vec_get_2,
        file: "vec-get.snek",
        input: "4",
        expected: "index out of bounds"
    },
    {
        name: error_vec_set_nil,
        file: "error-vec-set-nil.snek",
        expected: "invalid vector address"
    },
    {
        name: error_vec_set_1,
        file: "vec-set.snek",
        input: "-1",
        expected: "index out of bounds"
    },
    {
        name: error_vec_set_2,
        file: "vec-set.snek",
        input: "4",
        expected: "index out of bounds"
    },

    {
        name: error_vec_len,
        file: "error-vec-len.snek",
        expected: "invalid vector address"
    },
}

static_error_tests! {
    {
        name: parse_vec_invalid_1,
        file: "parse_vec_invalid_1.snek",
        expected: "Invalid"
    },
    {
        name: parse_vec_get_invalid_1,
        file: "parse_vec_get_invalid_1.snek",
        expected: "Invalid"
    },
    {
        name: parse_vec_set_invalid_1,
        file: "parse_vec_set_invalid_1.snek",
        expected: "Invalid"
    },
}
