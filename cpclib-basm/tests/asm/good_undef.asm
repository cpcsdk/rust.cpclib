my_label = 1

    ifndef my_label
        fail "my_label must exist"
    endif

    undef my_label

    ifdef my_label
        fail "my_label must not exist"
    endif