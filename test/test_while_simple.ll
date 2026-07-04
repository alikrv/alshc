; ModuleID = 'alsh'
source_filename = "alsh"

@str_chars = global [20 x i8] c"loop at iteration: \00"
@str_obj = global { i64, i64, ptr } { i64 19, i64 19, ptr @str_chars }
@format = global [4 x i8] c"%s\0A\00"

define i32 @main(i32 %0, ptr %1) {
entry:
  %i = alloca i32, align 4
  store i32 0, ptr %i, align 4
  br label %while_cond

while_cond:                                       ; preds = %while_body, %entry
  %i1 = load i32, ptr %i, align 4
  %cmp = icmp slt i32 %i1, 3
  br i1 %cmp, label %while_body, label %while_end

while_body:                                       ; preds = %while_cond
  %i2 = load i32, ptr %i, align 4
  %widen = zext i32 %i2 to i64
  %to_str = call ptr @alsh_int_to_str(i64 %widen)
  %concat = call ptr @alsh_str_concat(ptr @str_obj, ptr %to_str)
  %data_ptr = getelementptr inbounds nuw { i64, i64, ptr }, ptr %concat, i32 0, i32 2
  %data_load = load ptr, ptr %data_ptr, align 8
  %println = call i32 (ptr, ...) @printf(ptr @format, ptr %data_load)
  %i3 = load i32, ptr %i, align 4
  %add = add i32 %i3, 1
  store i32 %add, ptr %i, align 4
  br label %while_cond

while_end:                                        ; preds = %while_cond
  ret i32 0
}

declare i32 @printf(ptr, ...)

declare ptr @alsh_int_to_str(i64)

declare ptr @alsh_str_concat(ptr, ptr)
