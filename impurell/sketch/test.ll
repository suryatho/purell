; header
declare void @imp_show(ptr)
declare ptr @imp_num(ptr)
declare ptr @imp_add(ptr)

declare void @cap_push(ptr)
declare void @cap_clear()
declare ptr @cap_get(i32)

%impfn = type ptr(ptr)*

%expr0 = constant [18 x i8] c"(\x.\y.+ x y) 3 5\0A"
%expr1 = constant [10 x i8] c"(\x.x) 42\0A"
%expr2 = constant [29 x i8] c"(\x.\y.\z.+ x (+ y z)) 1 2 3\0A"

; (\y.+ 3 y) 5
; -> + 3 5 -> 8
define ptr @lambda_0.1(ptr %y) {
    %cap_0 = call ptr @cap_get(i32 0)
    %add_0fn = call ptr @imp_add(ptr %cap_0)
    %result = tail call ptr %add_0fn(ptr %y)
    ret ptr %result
}

; (\x.\y.+ x y) 3
; -> (\y.+ 3 y)
define ptr @lambda_0.0(ptr %x) {
    call void @cap_push(ptr %x)
    ret ptr @lambda_0.1
}

; compile: (\x.\y.+ x y) 3 5
; outputs: 8
define void @lambda_0() {
    %num_3 = call ptr @imp_num(ptr 3)
    %res.0 = call ptr @lambda_0.0(ptr %num_3)

    %num_5 = call ptr @imp_num(ptr 5)
    %res.1 = tail call ptr %res.0(ptr %num_5)

    call void @imp_show(ptr %res.1)
    call void @cap_clear()
    ret void
}

define ptr @lambda_1.0(ptr %x) {
    ret ptr %x
}

; (\x.x) 42
; -> 42
define void @lambda_1() {
    %num_42 = call ptr @imp_num(ptr 42)
    %res.0 = tail call ptr @lambda_1.0(ptr %num_42)

    call void @imp_show(ptr %res.0)
    ret void
}

define ptr @lambda_2.2(ptr %z) {
    %y = call ptr @cap_get(i32 1)
    %add_0.raw = call ptr @imp_add(ptr %y)
    %res.0 = tail call ptr %add_0.raw(ptr %z)

    %x = call ptr @cap_get(i32 0)
    %add_1.raw = call ptr @imp_add(ptr %x)
    %res_1 = tail call ptr %add_1.raw(ptr %res.0)
    ret ptr %res_1
}

define ptr @lambda_2.1(ptr %y) {
    call void @cap_push(ptr %y)
    ret ptr @lambda_2.2
}

define ptr @lambda_2.0(ptr %x) {
    call void @cap_push(ptr %x)
    ret ptr @lambda_2.1
}

; compile: (\x.\y.\z.+ x (+ y z)) 1 2 3
; outputs: 6
define void @lambda_2() {
    %num_1 = call ptr @imp_num(ptr 1)
    %res.0 = call ptr @lambda_2.0(ptr %num_1)
    ; (\y.\z.+ 1 (+ y z))

    ; apply 2
    %num_2 = call ptr @imp_num(ptr 2)
    %res.1 = call ptr %res.0(ptr %num_2)
    ; (\z.+ 1 (+ 2 z))

    ; apply 3 -> + 1 (+ 2 3) -> + 1 5 -> 6
    %num_3 = call ptr @imp_num(ptr 3)
    %res.2 = tail call ptr %res.1(ptr %num_3)
    ; 6

    call void @imp_show(ptr %res.2)
    call void @cap_clear()
    ret void
}

define i32 @imp_start() {
    call void @lambda_0()
    call void @lambda_1()
    call void @lambda_2()
    ret i32 0
}
