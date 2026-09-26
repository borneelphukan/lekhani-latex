content = open('gui/src/app/layout/menubar.rs').read()
head_start = content.find("<<<<<<< HEAD:gui/src/app/layout/menubar.rs")
while head_start != -1:
    mid = content.find("=======", head_start)
    end = content.find(">>>>>>> main:src/app/layout/menubar.rs", mid)
    if mid != -1 and end != -1:
        head_content = content[head_start + len("<<<<<<< HEAD:gui/src/app/layout/menubar.rs"):mid]
        content = content[:head_start] + head_content + content[end + len(">>>>>>> main:src/app/layout/menubar.rs"):]
    head_start = content.find("<<<<<<< HEAD:gui/src/app/layout/menubar.rs")

open('gui/src/app/layout/menubar.rs', 'w').write(content)
