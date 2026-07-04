; ModuleID = 'alsh'
source_filename = "alsh"

@str_chars = global [6 x i8] c"hello\00"
@str_obj = global { i64, i64, ptr } { i64 5, i64 5, ptr @str_chars }
@format = global [4 x i8] c"%s\0A\00"
@str_chars.1 = global [6 x i8] c"hello\00"
@str_obj.2 = global { i64, i64, ptr } { i64 5, i64 5, ptr @str_chars.1 }
@format.3 = global [4 x i8] c"%s\0A\00"

define i32 @main(i32 %0, ptr %1) {
entry:
  %data_load = load ptr, ptr getelementptr inbounds nuw ({ i64, i64, ptr }, ptr @str_obj, i32 0, i32 2), align 8
  %println = call i32 (ptr, ...) @printf(ptr @format, ptr %data_load)
  %data_load1 = load ptr, ptr getelementptr inbounds nuw ({ i64, i64, ptr }, ptr @str_obj.2, i32 0, i32 2), align 8
  %println2 = call i32 (ptr, ...) @printf(ptr @format.3, ptr %data_load1)
  ret i32 0
}

declare i32 @printf(ptr, ...)
