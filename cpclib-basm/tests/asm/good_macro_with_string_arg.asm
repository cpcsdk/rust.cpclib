macro m1 content
  assert {content} == "test"
  assert string_get({content}, 0) == "t"
  assert string_get({content}, 1) == "e"
  assert string_get({content}, 2) == "s"
  assert string_get({content}, 3) == "t"

	repeat string_len({content}), i, 0
    assert string_get({content}, {i}) == string_get("test", {i})
  endr

endm
m1("test")


macro m2 content
  content2={content}
  assert content2 == "test"
  assert string_get({content}, 0) == "t"
  assert string_get({content}, 1) == "e"
  assert string_get({content}, 2) == "s"
  assert string_get({content}, 3) == "t"
  assert string_get(content2, 0) == "t"
  assert string_get(content2, 1) == "e"
  assert string_get(content2, 2) == "s"
  assert string_get(content2, 3) == "t"

  repeat string_len({content}), i, 0
    assert string_get({content}, {i}) == string_get("test", {i})
    assert string_get(content2, {i}) == string_get("test", {i})
  endr
endm
m2("test")


macro m3 content
  @content3={content}
  assert @content3 == "test"
  assert string_get({content}, 0) == "t"
  assert string_get({content}, 1) == "e"
  assert string_get({content}, 2) == "s"
  assert string_get({content}, 3) == "t"
  assert string_get(@content3, 0) == "t"
  assert string_get(@content3, 1) == "e"
  assert string_get(@content3, 2) == "s"
  assert string_get(@content3, 3) == "t"

  repeat string_len({content}), i, 0
    assert string_get({content}, {i}) == string_get("test", {i})
    assert string_get(@content3, {i}) == string_get("test", {i})
  endr

endm
m3("test")


macro m4 content4
  @content4={content4}
  assert @content4 == "test"
  assert string_get({content4}, 0) == "t"
  assert string_get({content4}, 1) == "e"
  assert string_get({content4}, 2) == "s"
  assert string_get({content4}, 3) == "t"
  assert string_get(@content4, 0) == "t"
  assert string_get(@content4, 1) == "e"
  assert string_get(@content4, 2) == "s"
  assert string_get(@content4, 3) == "t"

  repeat string_len({content4}), i, 0
    assert string_get({content4}, {i}) == string_get("test", {i})
    assert string_get(@content4, {i}) == string_get("test", {i})
  endr
endm
m4("test")


