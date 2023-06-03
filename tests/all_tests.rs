mod infra;
success_tests! {
    {
        name: simple_examples,
        file: "simple_examples.snek",
        expected: "1\n2\n(tuple 1 2)\n3\n4\n5\n(tuple 3 4 5)\n(tuple (tuple 1 2) nil)\n(tuple (tuple 1 2) nil)"
    },
    {
        name: points,
        file: "points.snek",
        expected: "(tuple 5 10)\n(tuple 30 60)\n(tuple 35 70)\n(tuple 35 70)"
    },
    {
        name: bst,
        file: "bst.snek",
        expected: "(tuple 50 nil nil)\n(tuple 50 (tuple 25 nil nil) nil)\n(tuple 50 (tuple 25 nil nil) (tuple 75 nil nil))\n(tuple 50 (tuple 25 nil nil) (tuple 75 nil (tuple 100 nil nil)))\n(tuple 50 (tuple 25 (tuple 0 nil nil) nil) (tuple 75 nil (tuple 100 nil nil)))\ntrue\ntrue\ntrue\nfalse\nfalse\nfalse"
    },



}

runtime_error_tests! {
    {
        name: error_bounds_1,
        file: "error-bounds.snek",
        input: "-1",
        expected: "an error occurred: index out of bounds"
    },
    {
        name: error_bounds_2,
        file: "error-bounds.snek",
        input: "5",
        expected: "an error occurred: index out of bounds"
    },
    {
        name: error_tag,
        file: "error-tag.snek",
        expected: "an error occurred: invalid tuple address"
    },
    {
        name: error3,
        file: "error3.snek",
        expected: "an error occurred: invalid tuple offset"
    },
}

static_error_tests! {
    {
        name: parse_index_invalid_1,
        file: "parse_index_invalid_1.snek",
        expected: "",
    },
    {
        name: parse_index_invalid_2,
        file: "parse_index_invalid_2.snek",
        expected: "",
    },
    {
        name: parse_index_invalid_3,
        file: "parse_index_invalid_3.snek",
        expected: "",
    },
    {
        name: parse_tuple_invalid_1,
        file: "parse_tuple_invalid_1.snek",
        expected: "",
    },
    {
        name: parse_tuple_invalid_2,
        file: "parse_tuple_invalid_2.snek",
        expected: "",
    },

}
