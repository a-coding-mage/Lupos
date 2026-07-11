rules
	our current target is x86_64 generic config
	all code must be 1 to 1 with vendor/linux, we only allow a different implementation when rust can't reproduce the behavior from the c source.
	drivers are built from the original c source, and loaded from lupos kernel.
	avoid regression, use the testing tools from vendor/linux to evaluate our implementation, avoid making unit tests yourself, just consume the already field proofen tests from vendor/linux.
	agents cannot directly create branches or commit code without approval.
	always say the true.
	our abi should be exactly the same from vendor/linux.
	our behavior should be exactly the same form vendor/linux. 
	if you edited a file, you must ensure it's parity with vendor/linux c equivalent.
	if a task require touch multiple parts of the kernel where work can be made parallel, spawn agents to do that specific task, then evaluate them as they arrive, sub agents can't run the build, it should be done by the main agent.
