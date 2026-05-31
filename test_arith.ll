; ModuleID = 'alsh'
source_filename = "alsh"

@cmd = global [0 x i8] zeroinitializer
@format = global [5 x i8] c"%ld\0A\00"
@format.1 = global [5 x i8] c"%ld\0A\00"
@format.2 = global [5 x i8] c"%ld\0A\00"
@format.3 = global [5 x i8] c"%ld\0A\00"

define i32 @main() {
entry:
  %call = call i32 (ptr, ...) @printf(ptr @cmd)
  %x = alloca i64, align 8
  store i64 10, ptr %x, align 4
  %y = alloca i64, align 8
  store i64 3, ptr %y, align 4
  %x1 = load i64, ptr %x, align 4
  %y2 = load i64, ptr %y, align 4
  %add = add i64 %x1, %y2
  %add3 = alloca i64, align 8
  store i64 %add, ptr %add3, align 4
  %x4 = load i64, ptr %x, align 4
  %y5 = load i64, ptr %y, align 4
  %sub = sub i64 %x4, %y5
  %sub6 = alloca i64, align 8
  store i64 %sub, ptr %sub6, align 4
  %x7 = load i64, ptr %x, align 4
  %y8 = load i64, ptr %y, align 4
  %mul = mul i64 %x7, %y8
  %mul9 = alloca i64, align 8
  store i64 %mul, ptr %mul9, align 4
  %x10 = load i64, ptr %x, align 4
  %y11 = load i64, ptr %y, align 4
  %div = sdiv i64 %x10, %y11
  %div12 = alloca i64, align 8
  store i64 %div, ptr %div12, align 4
  %add13 = load i64, ptr %add3, align 4
  %println = call i32 (ptr, ...) @printf(ptr @format, i64 %add13)
  %sub14 = load i64, ptr %sub6, align 4
  %println15 = call i32 (ptr, ...) @printf(ptr @format.1, i64 %sub14)
  %mul16 = load i64, ptr %mul9, align 4
  %println17 = call i32 (ptr, ...) @printf(ptr @format.2, i64 %mul16)
  %div18 = load i64, ptr %div12, align 4
  %println19 = call i32 (ptr, ...) @printf(ptr @format.3, i64 %div18)
  ret i32 0
}

declare i32 @printf(ptr, ...)
