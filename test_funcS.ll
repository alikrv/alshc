; ModuleID = 'alsh'
source_filename = "alsh"

@str_chars = global [22 x i8] c"testing foo with true\00"
@str_obj = global { i64, i64, ptr } { i64 21, i64 21, ptr @str_chars }
@str_chars.1 = global [23 x i8] c"testing foo with false\00"
@str_obj.2 = global { i64, i64, ptr } { i64 22, i64 22, ptr @str_chars.1 }

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
  call void @alsh_std_println(ptr %data_load)
  %call = call i32 @foo_func(i32 2, i32 1)
  store i32 %call, ptr %result, align 4
  %result1 = load i32, ptr %result, align 4
  %widen = zext i32 %result1 to i64
  %to_str = call ptr @alsh_int_to_str(i64 %widen)
  %data_ptr = getelementptr inbounds nuw { i64, i64, ptr }, ptr %to_str, i32 0, i32 2
  %data_load2 = load ptr, ptr %data_ptr, align 8
  call void @alsh_std_println(ptr %data_load2)
  %data_load3 = load ptr, ptr getelementptr inbounds nuw ({ i64, i64, ptr }, ptr @str_obj.2, i32 0, i32 2), align 8
  call void @alsh_std_println(ptr %data_load3)
  %call4 = call i32 @foo_func(i32 2, i32 0)
  store i32 %call4, ptr %result2, align 4
  %result25 = load i32, ptr %result2, align 4
  %widen6 = zext i32 %result25 to i64
  %to_str7 = call ptr @alsh_int_to_str(i64 %widen6)
  %data_ptr8 = getelementptr inbounds nuw { i64, i64, ptr }, ptr %to_str7, i32 0, i32 2
  %data_load9 = load ptr, ptr %data_ptr8, align 8
  call void @alsh_std_println(ptr %data_load9)
  ret i32 0
}

declare void @alsh_std_println(ptr)

declare ptr @alsh_int_to_str(i64)
