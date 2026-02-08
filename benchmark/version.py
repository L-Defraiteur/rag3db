import os

base_dir = os.path.dirname(os.path.realpath(__file__))

def _get_rag3db_version():
    cmake_file = os.path.join(base_dir, '..', 'CMakeLists.txt')
    with open(cmake_file) as f:
        for line in f:
            if line.startswith('project(Rag3db VERSION'):
                return line.split(' ')[2].strip()
    return "0"

if __name__ == "__main__":
    print(_get_rag3db_version())
