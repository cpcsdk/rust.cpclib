
    function koch initial_koch
        new_koch = ""
        repeat string_len({initial_koch}), i, 0
            new_koch = new_koch + koch_transform(list_get({initial_koch}, {i}))
        endr
        return new_koch 
    endf

    function koch_transform symbol
        switch symbol
            case 'F'
                return "F+F-F-F+F"
            default
                return symbol
        endswitch
    endf

    function build_koch depth
        if {depth} == 0
            return "F"
        else
            return koch(build_koch({depth} - 1))
        endif
    endf

    print build_koch(0)
    print build_koch(1)