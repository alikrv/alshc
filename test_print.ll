; ModuleID = 'alsh'
source_filename = "alsh"

@str_chars = global [24 x i8] c"=== Testing println ===\00"
@str_obj = global { i64, i64, ptr } { i64 23, i64 23, ptr @str_chars }
@str_chars.1 = global [14 x i8] c"Hello, World!\00"
@str_obj.2 = global { i64, i64, ptr } { i64 13, i64 13, ptr @str_chars.1 }
@str_chars.3 = global [15 x i8] c"This is a test\00"
@str_obj.4 = global { i64, i64, ptr } { i64 14, i64 14, ptr @str_chars.3 }

define i32 @main(i32 %0, ptr %1) {
entry:
  %data_load = load ptr, ptr getelementptr inbounds nuw ({ i64, i64, ptr }, ptr @str_obj, i32 0, i32 2), align 8
  call void @alsh_std_println(ptr %data_load)
  %data_load1 = load ptr, ptr getelementptr inbounds nuw ({ i64, i64, ptr }, ptr @str_obj.2, i32 0, i32 2), align 8
  call void @alsh_std_println(ptr %data_load1)
  %data_load2 = load ptr, ptr getelementptr inbounds nuw ({ i64, i64, ptr }, ptr @str_obj.4, i32 0, i32 2), align 8
  call void @alsh_std_println(ptr %data_load2)
  ret i32 0
}

declare void @alsh_std_println(ptr)
