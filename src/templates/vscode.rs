use crate::languages::Language;

pub fn generate_vscode_settings(language: Language) -> String {
	let extension = match language {
		Language::C => "c",
		Language::Cpp => "cpp",
	};

	format!(
		"{{\n\
        \t\"[{}]\": {{\n\
        \t\t\"editor.defaultFormatter\": \"ms-vscode.cpptools\",\n\
        \t\t\"editor.formatOnSave\": true,\n\
        \t\t\"editor.rulers\": [100],\n\
        \t\t\"editor.tabSize\": 4,\n\
        \t\t\"editor.insertSpaces\": false\n\
        \t}}\n\
        }}\n",
		extension
	)
}

pub fn generate_vscode_launch_config(project_name: &str) -> String {
	format!(
		"{{\n\
        \t\"version\": \"0.2.0\",\n\
        \t\"configurations\": [\n\
        \t\t{{\n\
        \t\t\t\"name\": \"C/C++ Debug\",\n\
        \t\t\t\"type\": \"cppdbg\",\n\
        \t\t\t\"request\": \"launch\",\n\
        \t\t\t\"program\": \"${{workspaceFolder}}/bin/{}\",\n\
        \t\t\t\"args\": [],\n\
        \t\t\t\"stopAtEntry\": false,\n\
        \t\t\t\"cwd\": \"${{workspaceFolder}}\",\n\
        \t\t\t\"environment\": [],\n\
        \t\t\t\"externalConsole\": false,\n\
        \t\t\t\"MIMode\": \"gdb\",\n\
        \t\t\t\"preLaunchTask\": \"build\",\n\
        \t\t\t\"setupCommands\": [\n\
        \t\t\t\t{{\n\
        \t\t\t\t\"description\": \"Enable pretty-printing for gdb\",\n\
        \t\t\t\t\"text\": \"-enable-pretty-printing\",\n\
        \t\t\t\t\"ignoreFailures\": true\n\
        \t\t\t\t}}\n\
        \t\t\t]\n\
        \t\t}}\n\
        \t]\n\
        }}\n",
		project_name
	)
}

pub fn generate_vscode_tasks_config() -> String {
	"{\n\
    \t\"version\": \"2.0.0\",\n\
    \t\"tasks\": [\n\
    \t\t{\n\
    \t\t\t\"label\": \"build\",\n\
    \t\t\t\"type\": \"shell\",\n\
    \t\t\t\"command\": \"mkdir -p build bin && cd build && cmake -DCMAKE_BUILD_TYPE=Debug .. && cmake --build .\",\n\
    \t\t\t\"problemMatcher\": [\"$gcc\"],\n\
    \t\t\t\"group\": {\n\
    \t\t\t\t\"kind\": \"build\",\n\
    \t\t\t\t\"isDefault\": true\n\
    \t\t\t}\n\
    \t\t},\n\
    \t\t{\n\
    \t\t\t\"label\": \"rebuild\",\n\
    \t\t\t\"type\": \"shell\",\n\
    \t\t\t\"command\": \"rm -rf build bin && mkdir -p build bin && cd build && cmake .. && cmake --build .\",\n\
    \t\t\t\"problemMatcher\": [\"$gcc\"]\n\
    \t\t}\n\
    \t]\n\
    }\n"
        .to_string()
}
