use crate::languages::Language;

pub fn generate_readme(project_name: &str, language: Language) -> String {
	let lang_name = match language {
		Language::C => "C",
		Language::Cpp => "C++",
	};

	format!(
		"# {}\n\n\
        A {} project created with [sticks](https://github.com/mAmineChniti/sticks).\n\n\
        ## Building\n\n\
        ### Using CMake (Recommended)\n\
        ```bash\n\
        mkdir build\n\
        cd build\n\
        cmake ..\n\
        cmake --build .\n\
        ```\n\n\
        ### Using Makefile\n\
        ```bash\n\
        make\n\
        make run\n\
        ```\n\n\
        ### Using Make with Debug\n\
        ```bash\n\
        make debug\n\
        ```\n\n\
        ## Project Structure\n\n\
        ```\n\
        {}/\n\
        ├── src/              # Source files\n\
        ├── include/          # Header files (if applicable)\n\
        ├── bin/              # Final compiled binaries (generated)\n\
        ├── build/            # Object files and build artifacts (generated)\n\
        ├── CMakeLists.txt    # CMake configuration\n\
        ├── Makefile          # Makefile configuration\n\
        └── README.md         # This file\n\
        ```\n\n\
        ## Adding Dependencies\n\n\
        ### Using sticks\n\
        ```bash\n\
        sticks add libcurl openssl\n\
        ```\n\n\
        ### Adding Source Files\n\
        ```bash\n\
        sticks src utils network\n\
        ```\n\n\
        ## License\n\n\
        This project is licensed under the MIT License.\n",
		project_name, lang_name, project_name
	)
}
