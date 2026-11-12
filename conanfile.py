from conan import ConanFile

class Pkg(ConanFile):
    requires = "zlib/1.2.13 ü"
