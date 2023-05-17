
    section .text
    global our_code_starts_here
    extern snek_error
    extern snek_print
	error_numeric_overflow:
		mov rdi, 1
		push rsp
		call snek_error
	error_invalid_type:
		mov rdi, 2
		push rsp
		call snek_error
	error_index_out_of_bounds:
		mov rdi, 3
		push rsp
		call snek_error
	error_not_heap_address:
		mov rdi, 4
		push rsp
		call snek_error
	error_not_index_offset:
		mov rdi, 5
		push rsp
		call snek_error
	loopfn:
		push rbx
		push rbp
		mov rbp, rsp
		mov rax, [rbp + 24]
		test rax, 1
		jnz error_invalid_type
		mov [rbp - 24], rax
		mov rax, [rbp + 32]
		test rax, 1
		jnz error_invalid_type
		add rax, [rbp - 24]
		jo error_numeric_overflow
		test rax, 1
		jnz error_invalid_type
		mov [rbp - 24], rax
		mov rax, [rbp + 40]
		test rax, 1
		jnz error_invalid_type
		add rax, [rbp - 24]
		jo error_numeric_overflow
		pop rbp
		pop rbx
		ret
	our_code_starts_here:
		mov r15, rsi
		mov r13, rsi
		mov r14, rdx
		sub rsp, 8
		push rbp
		mov rbp, rsp
		mov rax, 0
		mov [rbp - 16], rax
		mov rax, 10
		mov [rbp - 24], rax
		mov rax, 20
		mov [rbp - 32], rax
		mov rax, 40
		mov [rbp - 40], rax
		loop_0:
			mov rax, [rbp - 16]
			mov [rbp - 48], rax
			mov rax, rdi
			mov rbx, rax
			or rbx, [rbp - 48]
			test rbx, 1
			jne error_invalid_type
			cmp [rbp - 48], rax
			mov rbx, 7
			mov rax, 3
			cmovge rax, rbx
			cmp rax, 3
			je ifelse_3
				mov rax, [rbp - 16]
				jmp endloop_1
				jmp ifend_2
			ifelse_3:
				mov [rbp - 48], rdi
				mov [rbp - 56], rsi
				mov [rbp - 64], rdx
				mov rax, [rbp - 40]
				mov [rbp - 72], rax
				mov rax, [rbp - 32]
				mov [rbp - 80], rax
				mov rax, [rbp - 16]
				mov [rbp - 88], rax
				sub rsp, 88
				call loopfn
				mov rdi, [rbp - 48]
				mov rsi, [rbp - 56]
				mov rdx, [rbp - 64]
				add rsp, 88
				sub rsp, 8
				mov [rbp - 56], rdi
				mov [rbp - 64], rsi
				mov [rbp - 72], rdx
				mov rdi, rax
				sub rsp, 72
				call snek_print
				add rsp, 72
				mov rdi, [rbp - 56]
				mov rsi, [rbp - 64]
				mov rdx, [rbp - 72]
				mov rax, [rbp - 16]
				mov rbx, rax
				not rbx
				and rbx, 1
				cmp rbx, 1
				jne error_invalid_type
				add rax, 2
				jo error_numeric_overflow
				mov [rbp - 16], rax
			ifend_2:
		jmp loop_0
		endloop_1:
		pop rbp
		add rsp, 8
		ret
    