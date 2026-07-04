; ModuleID = 'alsh'
source_filename = "alsh"

@str_chars = global [8 x i8] c"Hello, \00"
@str_obj = global { i64, i64, ptr } { i64 7, i64 7, ptr @str_chars }
@str_chars.1 = global [2 x i8] c"!\00"
@str_obj.2 = global { i64, i64, ptr } { i64 1, i64 1, ptr @str_chars.1 }
@str_chars.3 = global [37 x i8] c"=== Testing Functions with Types ===\00"
@str_obj.4 = global { i64, i64, ptr } { i64 36, i64 36, ptr @str_chars.3 }
@str_chars.5 = global [6 x i8] c"Alice\00"
@str_obj.6 = global { i64, i64, ptr } { i64 5, i64 5, ptr @str_chars.5 }
@str_chars.7 = global [4 x i8] c"Bob\00"
@str_obj.8 = global { i64, i64, ptr } { i64 3, i64 3, ptr @str_chars.7 }
@str_chars.9 = global [13 x i8] c"Flag is true\00"
@str_obj.10 = global { i64, i64, ptr } { i64 12, i64 12, ptr @str_chars.9 }
@str_chars.11 = global [14 x i8] c"Flag is false\00"
@str_obj.12 = global { i64, i64, ptr } { i64 13, i64 13, ptr @str_chars.11 }
@str_chars.13 = global [4 x i8] c" + \00"
@str_obj.14 = global { i64, i64, ptr } { i64 3, i64 3, ptr @str_chars.13 }
@str_chars.15 = global [4 x i8] c" = \00"
@str_obj.16 = global { i64, i64, ptr } { i64 3, i64 3, ptr @str_chars.15 }

define i32 @greet(ptr %0) {
entry:
  %name = alloca ptr, align 8
  store ptr %0, ptr %name, align 8
  %name1 = load ptr, ptr %name, align 8
  %concat = call ptr @alsh_str_concat(ptr @str_obj, ptr %name1)
  %concat2 = call ptr @alsh_str_concat(ptr %concat, ptr @str_obj.2)
  %data_ptr = getelementptr inbounds nuw { i64, i64, ptr }, ptr %concat2, i32 0, i32 2
  %data_load = load ptr, ptr %data_ptr, align 8
  call void @alsh_std_println(ptr %data_load)
  ret i32 0
}

define i32 @main(i32 %0, ptr %1) {
entry:
  %data_load = load ptr, ptr getelementptr inbounds nuw ({ i64, i64, ptr }, ptr @str_obj.4, i32 0, i32 2), align 8
  call void @alsh_std_println(ptr %data_load)
  %call = call i32 @greet(ptr @str_obj.6)
  %call1 = call i32 @greet(ptr @str_obj.8)
  %call2 = call i32 @add_and_print(i32 5, i32 3)
  %call3 = call i32 @add_and_print(i32 10, i32 20)
  %call4 = call i32 @print_bool(i32 1)
  %call5 = call i32 @print_bool(i32 0)
  ret i32 0
}

define i32 @print_bool(i32 %0) {
entry:
  %flag = alloca i32, align 4
  store i32 %0, ptr %flag, align 4
  %flag1 = load i32, ptr %flag, align 4
  %cond = icmp ne i32 %flag1, 0
  br i1 %cond, label %if_then, label %else_block

if_then:                                          ; preds = %entry
  %data_load = load ptr, ptr getelementptr inbounds nuw ({ i64, i64, ptr }, ptr @str_obj.10, i32 0, i32 2), align 8
  call void @alsh_std_println(ptr %data_load)
  br label %if_end

if_end:                                           ; preds = %else_block, %if_then
  ret i32 0

else_block:                                       ; preds = %entry
  %data_load2 = load ptr, ptr getelementptr inbounds nuw ({ i64, i64, ptr }, ptr @str_obj.12, i32 0, i32 2), align 8
  call void @alsh_std_println(ptr %data_load2)
  br label %if_end
}

define i32 @add_and_print(i32 %0, i32 %1) {
entry:
  %result = alloca i32, align 4
  %b = alloca i32, align 4
  %a = alloca i32, align 4
  store i32 %0, ptr %a, align 4
  store i32 %1, ptr %b, align 4
  %a1 = load i32, ptr %a, align 4
  %b2 = load i32, ptr %b, align 4
  %add = add i32 %a1, %b2
  store i32 %add, ptr %result, align 4
  %a3 = load i32, ptr %a, align 4
  %widen = zext i32 %a3 to i64
  %to_str = call ptr @alsh_int_to_str(i64 %widen)
  %concat = call ptr @alsh_str_concat(ptr %to_str, ptr @str_obj.14)
  %b4 = load i32, ptr %b, align 4
  %widen5 = zext i32 %b4 to i64
  %to_str6 = call ptr @alsh_int_to_str(i64 %widen5)
  %concat7 = call ptr @alsh_str_concat(ptr %concat, ptr %to_str6)
  %concat8 = call ptr @alsh_str_concat(ptr %concat7, ptr @str_obj.16)
  %result9 = load i32, ptr %result, align 4
  %widen10 = zext i32 %result9 to i64
  %to_str11 = call ptr @alsh_int_to_str(i64 %widen10)
  %concat12 = call ptr @alsh_str_concat(ptr %concat8, ptr %to_str11)
  %data_ptr = getelementptr inbounds nuw { i64, i64, ptr }, ptr %concat12, i32 0, i32 2
  %data_load = load ptr, ptr %data_ptr, align 8
  call void @alsh_std_println(ptr %data_load)
  %result13 = load i32, ptr %result, align 4
  ret i32 %result13
}

declare void @alsh_std_println(ptr)

declare ptr @alsh_str_concat(ptr, ptr)

declare ptr @alsh_int_to_str(i64)
