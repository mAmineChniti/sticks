pub fn generate_editorconfig() -> String {
	"root = true\n\
    \n\
    [*]\n\
    indent_style = tab\n\
    indent_size = 4\n\
    end_of_line = lf\n\
    charset = utf-8\n\
    trim_trailing_whitespace = true\n\
    insert_final_newline = true\n\
    \n\
    [*.md]\n\
    trim_trailing_whitespace = false\n"
		.to_string()
}
