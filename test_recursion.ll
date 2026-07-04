; ModuleID = 'alsh'
source_filename = "alsh"

@str_chars = global [36 x i8] c"=== Testing Recursive Functions ===\00"
@str_obj = global { i64, i64, ptr } { i64 35, i64 35, ptr @str_chars }
@str_chars.1 = global [24 x i8] c"Factorial calculations:\00"
@str_obj.2 = global { i64, i64, ptr } { i64 23, i64 23, ptr @str_chars.1 }
@str_chars.3 = global [16 x i8] c"factorial(3) = \00"
@str_obj.4 = global { i64, i64, ptr } { i64 15, i64 15, ptr @str_chars.3 }
@str_chars.5 = global [16 x i8] c"factorial(5) = \00"
@str_obj.6 = global { i64, i64, ptr } { i64 15, i64 15, ptr @str_chars.5 }
@str_chars.7 = global [16 x i8] c"factorial(4) = \00"
@str_obj.8 = global { i64, i64, ptr } { i64 15, i64 15, ptr @str_chars.7 }

define i32 @main(i32 %0, ptr %1) {
entry:
  %f3 = alloca i32, align 4
  %f2 = alloca i32, align 4
  %f1 = alloca i32, align 4
  %data_load = load ptr, ptr getelementptr inbounds nuw ({ i64, i64, ptr }, ptr @str_obj, i32 0, i32 2), align 8
  call void @alsh_std_println(ptr %data_load)
  %data_load1 = load ptr, ptr getelementptr inbounds nuw ({ i64, i64, ptr }, ptr @str_obj.2, i32 0, i32 2), align 8
  call void @alsh_std_println(ptr %data_load1)
  %call = call i32 @factorial(i32 3)
  store i32 %call, ptr %f1, align 4
  %f12 = load i32, ptr %f1, align 4
  %widen = zext i32 %f12 to i64
  %to_str = call ptr @alsh_int_to_str(i64 %widen)
  %concat = call ptr @alsh_str_concat(ptr @str_obj.4, ptr %to_str)
  %data_ptr = getelementptr inbounds nuw { i64, i64, ptr }, ptr %concat, i32 0, i32 2
  %data_load3 = load ptr, ptr %data_ptr, align 8
  call void @alsh_std_println(ptr %data_load3)
  %call4 = call i32 @factorial(i32 5)
  store i32 %call4, ptr %f2, align 4
  %f25 = load i32, ptr %f2, align 4
  %widen6 = zext i32 %f25 to i64
  %to_str7 = call ptr @alsh_int_to_str(i64 %widen6)
  %concat8 = call ptr @alsh_str_concat(ptr @str_obj.6, ptr %to_str7)
  %data_ptr9 = getelementptr inbounds nuw { i64, i64, ptr }, ptr %concat8, i32 0, i32 2
  %data_load10 = load ptr, ptr %data_ptr9, align 8
  call void @alsh_std_println(ptr %data_load10)
  %call11 = call i32 @factorial(i32 4)
  store i32 %call11, ptr %f3, align 4
  %f312 = load i32, ptr %f3, align 4
  %widen13 = zext i32 %f312 to i64
  %to_str14 = call ptr @alsh_int_to_str(i64 %widen13)
  %concat15 = call ptr @alsh_str_concat(ptr @str_obj.8, ptr %to_str14)
  %data_ptr16 = getelementptr inbounds nuw { i64, i64, ptr }, ptr %concat15, i32 0, i32 2
  %data_load17 = load ptr, ptr %data_ptr16, align 8
  call void @alsh_std_println(ptr %data_load17)
  ret i32 0
}

define i32 @factorial(i32 %0) {
entry:
  %result = alloca i32, align 4
  %n = alloca i32, align 4
  store i32 %0, ptr %n, align 4
  %n1 = load i32, ptr %n, align 4
  %cmp = icmp sle i32 %n1, 1
  br i1 %cmp, label %if_then, label %if_end

if_then:                                          ; preds = %entry
  ret i32 1
  br label %if_end

if_end:                                           ; preds = %if_then, %entry
  %n2 = load i32, ptr %n, align 4
  %n3 = load i32, ptr %n, align 4
  %sub = sub i32 %n3, 1
  %call = call i32 @factorial(i32 %sub)
  %mul = mul i32 %n2, %call
  store i32 %mul, ptr %result, align 4
  %result4 = load i32, ptr %result, align 4
  ret i32 %result4
}

declare void @alsh_std_println(ptr)

declare ptr @alsh_int_to_str(i64)

declare ptr @alsh_str_concat(ptr, ptr)
