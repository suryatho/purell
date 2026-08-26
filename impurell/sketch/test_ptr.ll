@fmt = constant [4 x i8] c"%d\0A\00"

declare i32 @printf(ptr, ...)

define void @printlow4(ptr %x) {
    %addr = ptrtoint ptr %x to i64
    %low4 = and i64 %addr, 7

    %fmt_ptr = getelementptr [4 x i8], ptr @fmt, i32 0, i32 0
    call i32 (ptr, ...) @printf(ptr %fmt_ptr, i64 %low4)
    ret void
}

define i32 @main() {
    %x = alloca i64
    call void @printlow4(ptr %x)

    %y = alloca i64
    call void @printlow4(ptr %y)

    %test = alloca i32, align 8
    call void @printlow4(ptr %test)

    ret i32 0
}
