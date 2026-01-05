import os

# Имя итогового файла
OUTPUT_FILE = "full_code_archive.txt"

# Папки, которые нужно игнорировать
# Добавил 'target' (Rust), 'build', 'dist', 'obj', 'bin' (C++/C# и др.)
IGNORE_DIRS = {
    ".git", ".idea", "__pycache__", "venv", "env", "node_modules", ".vscode",
    "target", "build", "dist", "bin", "obj", ".gradle", ".mypy_cache"
}

# Расширения файлов, которые точно нужно пропускать
# Добавил .rlib (Rust библиотеки), .pdb (отладка), .d (зависимости), .lock (иногда мешают в target)
IGNORE_EXTENSIONS = {
    ".exe", ".dll", ".so", ".o", ".a", ".bin", ".png", ".jpg", ".jpeg", ".gif", 
    ".ico", ".zip", ".tar", ".gz", ".pyc", ".rlib", ".pdb", ".d", ".timestamp", 
    ".suo", ".user", ".rmeta", ".winmd"
}

def is_text_file(filepath):
    """
    Проверяет, является ли файл текстовым, пытаясь прочитать его.
    Если файл бинарный или имеет странную кодировку, он будет пропущен.
    """
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            f.read(1024)  # Читаем немного, чтобы проверить кодировку
        return True
    except (UnicodeDecodeError, PermissionError):
        return False

def merge_files():
    # Получаем путь к папке, где лежит скрипт
    current_dir = os.getcwd()
    
    print(f"Начинаю сканирование папки: {current_dir}")
    print(f"Результат будет записан в: {OUTPUT_FILE}")
    print("-" * 40)

    count = 0

    # Открываем итоговый файл на запись
    with open(OUTPUT_FILE, "w", encoding="utf-8") as outfile:
        # os.walk проходит по дереву каталогов
        for root, dirs, files in os.walk(current_dir):
            
            # Удаляем игнорируемые папки из списка, чтобы не заходить в них
            # Это предотвратит заход в глубокие папки типа target/debug/deps
            dirs[:] = [d for d in dirs if d not in IGNORE_DIRS]

            for file in files:
                # Полный путь к файлу
                filepath = os.path.join(root, file)
                
                # Пропускаем сам итоговый файл и сам скрипт
                if file == OUTPUT_FILE or file == os.path.basename(__file__):
                    continue

                # Пропускаем по расширению
                _, ext = os.path.splitext(file)
                if ext.lower() in IGNORE_EXTENSIONS:
                    continue
                
                # Дополнительная проверка: игнорируем файлы, начинающиеся с точки (скрытые), кроме .gitignore и т.п.
                if file.startswith(".") and file not in [".gitignore", ".env"]:
                    continue

                # Проверка: текстовый ли файл (чтобы не копировать мусор)
                if is_text_file(filepath):
                    try:
                        with open(filepath, "r", encoding="utf-8") as infile:
                            content = infile.read()
                            
                            # Относительный путь для красивого заголовка
                            relative_path = os.path.relpath(filepath, current_dir)
                            
                            # Записываем заголовок
                            outfile.write("\n" + "="*50 + "\n")
                            outfile.write(f"ФАЙЛ: {relative_path}\n")
                            outfile.write("="*50 + "\n\n")
                            
                            # Записываем содержимое
                            outfile.write(content)
                            outfile.write("\n") # Отступ после файла
                            
                            print(f"Скопирован: {relative_path}")
                            count += 1
                    except Exception as e:
                        print(f"Ошибка при чтении {file}: {e}")

    print("-" * 40)
    print(f"Готово! Обработано файлов: {count}")
    print(f"Полный архив сохранен в {OUTPUT_FILE}")

if __name__ == "__main__":
    merge_files()