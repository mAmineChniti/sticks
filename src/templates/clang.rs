use crate::languages::Language;

pub fn generate_clang_format_config(language: Language) -> String {
	match language {
		Language::C => "---\n\
            Language: C\n\
            Standard: C11\n\
            IndentWidth: 4\n\
            UseTab: ForContinuationAndIndentation\n\
            TabWidth: 4\n\
            ColumnLimit: 100\n\
            AllowShortFunctionsOnASingleLine: Empty\n\
            AllowShortIfStatementsOnASingleLine: Never\n\
            BreakBeforeBraces: Linux\n\
            SpaceAfterCStyleCast: true\n"
			.to_string(),
		Language::Cpp => "---\n\
            Language: Cpp\n\
            Standard: C++17\n\
            IndentWidth: 4\n\
            UseTab: ForContinuationAndIndentation\n\
            TabWidth: 4\n\
            ColumnLimit: 100\n\
            AllowShortFunctionsOnASingleLine: Empty\n\
            AllowShortIfStatementsOnASingleLine: Never\n\
            BreakBeforeBraces: Linux\n\
            SpaceAfterCStyleCast: true\n\
            Cpp11BracedListStyle: true\n"
			.to_string(),
	}
}
