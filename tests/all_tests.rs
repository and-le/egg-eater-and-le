mod infra;

success_tests! {
    {
        name: print1,
        file: "print1.snek",
        expected: "false\nfalse"
    },
    {
        name: print2,
        file: "print2.snek",
        expected: "1\n1"
    },
    {
        name: print3,
        file: "print3.snek",
        expected: "16\n16"
    },

    {
        name: identity1,
        file: "identity.snek",
        input: "5",
        expected: "5"
    },
    {
        name: identity2,
        file: "identity.snek",
        input: "true",
        expected: "true"
    },
    {
        name: two_params1,
        file: "two_params.snek",
        input: "100",
        expected: "110"
    },
    {
        name: two_params2,
        file: "two_params.snek",
        input: "-10",
        expected: "0"
    },
    {
        name: three_params1,
        file: "three_params.snek",
        expected: "117"
    },

    {
        name: four_params1,
        file: "four_params.snek",
        expected: "1117"
    },

    {
        name: many_prints1,
        file: "many_prints.snek",
        input: "10",
        expected: "60\n10\n600\n10\n10",
    },

    {
        name: fact,
        file: "fact.snek",
        input: "10",
        expected: "3628800",
    },
    {
        name: even_odd_1,
        file: "even_odd.snek",
        input: "10",
        expected: "10\ntrue\ntrue",
    },
    {
        name: even_odd_2,
        file: "even_odd.snek",
        input: "9",
        expected: "9\nfalse\nfalse",
    },
    {
        name: use_rdi_after_fun1,
        file: "use_rdi_after_fun.snek",
        input: "231",
        expected: "true\nfalse\nfalse\n231\n231",
    },
    {
        name: use_rdi_after_fun2,
        file: "use_rdi_after_fun.snek",
        input: "-231",
        expected: "false\ntrue\nfalse\n-231\n-231",
    },
    {
        name: use_rdi_after_fun3,
        file: "use_rdi_after_fun.snek",
        input: "0",
        expected: "false\nfalse\ntrue\n0\n0",
    },

    {
        name: fun_set,
        file: "fun_set.snek",
        expected: "21",
    },

    {
        name: no_arg_fun,
        file: "no_arg_fun.snek",
        expected: "60",
    },

    {
        name: fun_set_shadow,
        file: "fun_set_shadow.snek",
        expected: "30",
    },

    {
        name: let_main1,
        file: "let_main1.snek",
        expected: "60",
    },


    {
        name: sum,
        file: "sum.snek",
        expected: "55",
    },

    {
        name: fun_var,
        file: "fun_var.snek",
        expected: "6000",
    },

    {
        name: ackermann1,
        file: "ackermann.snek",
        input: "0",
        expected: "2",
    },
    {
        name: ackermann2,
        file: "ackermann.snek",
        input: "1",
        expected: "3",
    },
    {
        name: fun_param_same_name,
        file: "fun_param_same_name.snek",
        expected: "11",
    },
    {
        name: fun_in_loop,
        file: "fun_in_loop.snek",
        input: "3",
        expected: "30\n31\n32\n3",
    }

}

runtime_error_tests! {}

static_error_tests! {
    {
        name: duplicate_params,
        file: "duplicate_params.snek",
        expected: "Duplicate parameter",
    },

    {
        name: no_fun,
        file: "no_fun.snek",
        expected: "Invalid: undefined function nofun",
    },


    {
        name: input_in_fun,
        file: "input_in_fun.snek",
        expected: "Invalid: input can only be used in the main expression",
    },

    {
        name: too_many_args,
        file: "too_many_args.snek",
        expected: "Invalid: function foo called with 3 args, expected 2",
    },

    {
        name: too_few_args,
        file: "too_few_args.snek",
        expected: "Invalid: function foo called with 1 args, expected 2",
    },

    {
        name: keyword_param,
        file: "keyword_param.snek",
        expected: "Invalid: reserved keyword",
    },

    {
        name: keyword_fun,
        file: "keyword_fun.snek",
        expected: "Invalid: reserved keyword",
    },

}
