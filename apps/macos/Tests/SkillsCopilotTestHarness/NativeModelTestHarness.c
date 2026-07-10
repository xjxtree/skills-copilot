extern void SkillsCopilotRunNativeModelTests(void);

__attribute__((constructor)) static void run_skills_copilot_native_model_tests(void) {
    SkillsCopilotRunNativeModelTests();
}
