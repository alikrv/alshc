; ModuleID = 'alsh'
source_filename = "alsh"

@str_chars = global [22 x i8] c"testing foo with true\00"
@str_obj = global { i64, i64, ptr } { i64 21, i64 21, ptr @str_chars }
@format = global [4 x i8] c"%s\0A\00"
@format.1 = global [4 x i8] c"%s\0A\00"
@str_chars.2 = global [23 x i8] c"testing foo with false\00"
@str_obj.3 = global { i64, i64, ptr } { i64 22, i64 22, ptr @str_chars.2 }
@format.4 = global [4 x i8] c"%s\0A\00"
@format.5 = global [4 x i8] c"%s\0A\00"

define i32 @foo_func(i32 %0, i32 %1) {
entry:
  %result = alloca i32, align 4
  %actually_just_return_zero = alloca i32, align 4
  %num = alloca i32, align 4
  store i32 %0, ptr %num, align 4
  store i32 %1, ptr %actually_just_return_zero, align 4
  %num1 = load i32, ptr %num, align 4
  %mul = mul i32 %num1, 32
  store i32 %mul, ptr %result, align 4
  %actually_just_return_zero2 = load i32, ptr %actually_just_return_zero, align 4
  %cond = icmp ne i32 %actually_just_return_zero2, 0
  br i1 %cond, label %if_then, label %if_end

if_then:                                          ; preds = %entry
  ret i32 0
  br label %if_end

if_end:                                           ; preds = %if_then, %entry
  %result3 = load i32, ptr %result, align 4
  ret i32 %result3
}

define i32 @main(i32 %0, ptr %1) {
entry:
  %result2 = alloca i32, align 4
  %result = alloca i32, align 4
  %data_load = load ptr, ptr getelementptr inbounds nuw ({ i64, i64, ptr }, ptr @str_obj, i32 0, i32 2), align 8
  %println = call i32 (ptr, ...) @printf(ptr @format, ptr %data_load)
  %call = call i32 @foo_func(i32 2, i32 1)
  store i32 %call, ptr %result, align 4
  %result1 = load i32, ptr %result, align 4
  %widen = zext i32 %result1 to i64
  %to_str = call ptr @alsh_int_to_str(i64 %widen)
  %data_ptr = getelementptr inbounds nuw { i64, i64, ptr }, ptr %to_str, i32 0, i32 2
  %data_load2 = load ptr, ptr %data_ptr, align 8
  %println3 = call i32 (ptr, ...) @printf(ptr @format.1, ptr %data_load2)
  %data_load4 = load ptr, ptr getelementptr inbounds nuw ({ i64, i64, ptr }, ptr @str_obj.3, i32 0, i32 2), align 8
  %println5 = call i32 (ptr, ...) @printf(ptr @format.4, ptr %data_load4)
  %call6 = call i32 @foo_func(i32 2, i32 0)
  store i32 %call6, ptr %result2, align 4
  %result27 = load i32, ptr %result2, align 4
  %widen8 = zext i32 %result27 to i64
  %to_str9 = call ptr @alsh_int_to_str(i64 %widen8)
  %data_ptr10 = getelementptr inbounds nuw { i64, i64, ptr }, ptr %to_str9, i32 0, i32 2
  %data_load11 = load ptr, ptr %data_ptr10, align 8
  %println12 = call i32 (ptr, ...) @printf(ptr @format.5, ptr %data_load11)
  ret i32 0
}

declare i32 @printf(ptr, ...)

declare ptr @alsh_int_to_str(i64)
