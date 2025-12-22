use crate::languages::Language;

pub fn generate_gitignore(language: Language) -> String {
	match language {
		Language::C | Language::Cpp => "# Build artifacts\n\
            build/\n\
            cmake-build-*/\n\
            *.o\n\
            *.a\n\
            *.so\n\
            *.exe\n\
            *.dll\n\
            *.dylib\n\
            \n\
            # CMake\n\
            CMakeFiles/\n\
            CMakeCache.txt\n\
            cmake_install.cmake\n\
            Makefile\n\
            \n\
            # IDE\n\
            .vscode/\n\
            .idea/\n\
            *.swp\n\
            *.swo\n\
            *~\n\
            .DS_Store\n\
            \n\
            # Dependencies\n\
            conan.lock\n\
            vcpkg_installed/\n\
            \n\
            # Generated files\n\
            *.d\n\
            *.su\n\
            \n\
            # Binaries\n\
            bin/\n\
            dist/\n\
            \n\
            # Testing\n\
            test_*\n\
            my_project/\n"
			.to_string(),
	}
}

pub fn generate_gitattributes() -> &'static str {
	"* text=auto\n\
    *.c text eol=lf\n\
    *.cpp text eol=lf\n\
    *.h text eol=lf\n\
    *.cmake text eol=lf\n\
    Makefile text eol=lf\n\
    CMakeLists.txt text eol=lf\n\
    *.md text eol=lf\n"
}
