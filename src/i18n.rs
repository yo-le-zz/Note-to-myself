//! Traductions de l'interface. Chaque langue est un tableau dans l'ordre de `KEYS`
//! (la taille du tableau est vérifiée à la compilation).

use crate::app::Language;

pub const KEYS: [&str; 42] = [
    "new", "see", "write", "settings", "back", "save", "saving", "title_placeholder",
    "body_placeholder", "subtitle", "your_notes", "write_something", "language", "appearance",
    "dark_mode", "show_timestamps", "compact_mode", "confirm_delete", "font_size", "no_notes",
    "empty_note", "saved", "settings_saved", "save_error", "database_error", "read_error",
    "untitled", "updated", "home", "edit", "delete", "copy", "copied", "delete_confirm", "yes",
    "no", "deleted", "search", "no_results", "note_n", "characters", "delete_error",
];

type Table = [&'static str; 42];

const EN: Table = [
    "New note", "View notes", "Write a note", "Settings", "Back", "Save", "Saving…",
    "Give your note a title...", "Start writing here...", "A thought, just for yourself.",
    "Your notes", "Write something", "Language", "Appearance", "Dark mode", "Show timestamps",
    "Compact layout", "Confirm before deleting", "Editor font size",
    "No notes yet. Create your first one!", "Write something before saving.",
    "Note saved successfully.", "Settings saved.", "Could not save the note.",
    "Could not open the notes database.", "Could not read notes.", "Untitled", "Updated", "Home",
    "Edit", "Delete", "Copy text", "Copied to clipboard.", "Delete this note?", "Yes", "No",
    "Note deleted.", "Search notes...", "No matching notes.", "Note", "characters",
    "Could not delete the note.",
];

const FR: Table = [
    "Nouvelle note", "Voir les notes", "Écrire une note", "Paramètres", "Retour", "Enregistrer",
    "Enregistrement…", "Donne un titre à ta note...", "Commence à écrire ici...",
    "Une pensée, rien que pour toi.", "Tes notes", "Écrire quelque chose", "Langue", "Apparence",
    "Mode sombre", "Afficher les dates", "Interface compacte", "Confirmer avant suppression",
    "Taille du texte", "Pas encore de notes. Crée la première !",
    "Écris quelque chose avant d'enregistrer.", "Note enregistrée.", "Paramètres enregistrés.",
    "Impossible d'enregistrer la note.", "Impossible d'ouvrir la base de notes.",
    "Impossible de lire les notes.", "Sans titre", "Modifiée", "Accueil", "Modifier", "Supprimer",
    "Copier le texte", "Copié dans le presse-papiers.", "Supprimer cette note ?", "Oui", "Non",
    "Note supprimée.", "Rechercher une note...", "Aucune note trouvée.", "Note", "caractères",
    "Impossible de supprimer la note.",
];

const ES: Table = [
    "Nueva nota", "Ver notas", "Escribir una nota", "Ajustes", "Atrás", "Guardar", "Guardando…",
    "Ponle un título a tu nota...", "Empieza a escribir aquí...", "Un pensamiento, solo para ti.",
    "Tus notas", "Escribe algo", "Idioma", "Apariencia", "Modo oscuro", "Mostrar fechas",
    "Diseño compacto", "Confirmar antes de borrar", "Tamaño del texto",
    "Aún no hay notas. ¡Crea la primera!", "Escribe algo antes de guardar.", "Nota guardada.",
    "Ajustes guardados.", "No se pudo guardar la nota.", "No se pudo abrir la base de datos.",
    "No se pudieron leer las notas.", "Sin título", "Modificada", "Inicio", "Editar", "Eliminar",
    "Copiar texto", "Copiado al portapapeles.", "¿Eliminar esta nota?", "Sí", "No",
    "Nota eliminada.", "Buscar notas...", "No hay notas coincidentes.", "Nota", "caracteres",
    "No se pudo eliminar la nota.",
];

const DE: Table = [
    "Neue Notiz", "Notizen ansehen", "Notiz schreiben", "Einstellungen", "Zurück", "Speichern",
    "Speichern…", "Gib deiner Notiz einen Titel...", "Fang hier an zu schreiben...",
    "Ein Gedanke, nur für dich.", "Deine Notizen", "Schreib etwas", "Sprache", "Darstellung",
    "Dunkelmodus", "Zeitstempel anzeigen", "Kompaktes Layout", "Vor dem Löschen bestätigen",
    "Schriftgröße", "Noch keine Notizen. Erstelle deine erste!",
    "Schreib etwas, bevor du speicherst.", "Notiz gespeichert.", "Einstellungen gespeichert.",
    "Notiz konnte nicht gespeichert werden.", "Datenbank konnte nicht geöffnet werden.",
    "Notizen konnten nicht gelesen werden.", "Ohne Titel", "Geändert", "Start", "Bearbeiten",
    "Löschen", "Text kopieren", "In die Zwischenablage kopiert.", "Diese Notiz löschen?", "Ja",
    "Nein", "Notiz gelöscht.", "Notizen suchen...", "Keine passenden Notizen.", "Notiz",
    "Zeichen", "Notiz konnte nicht gelöscht werden.",
];

const IT: Table = [
    "Nuova nota", "Vedi le note", "Scrivi una nota", "Impostazioni", "Indietro", "Salva",
    "Salvataggio…", "Dai un titolo alla nota...", "Inizia a scrivere qui...",
    "Un pensiero, solo per te.", "Le tue note", "Scrivi qualcosa", "Lingua", "Aspetto",
    "Modalità scura", "Mostra data e ora", "Layout compatto", "Conferma prima di eliminare",
    "Dimensione del testo", "Ancora nessuna nota. Crea la prima!",
    "Scrivi qualcosa prima di salvare.", "Nota salvata.", "Impostazioni salvate.",
    "Impossibile salvare la nota.", "Impossibile aprire il database.",
    "Impossibile leggere le note.", "Senza titolo", "Modificata", "Home", "Modifica", "Elimina",
    "Copia testo", "Copiato negli appunti.", "Eliminare questa nota?", "Sì", "No",
    "Nota eliminata.", "Cerca note...", "Nessuna nota trovata.", "Nota", "caratteri",
    "Impossibile eliminare la nota.",
];

const PT: Table = [
    "Nova nota", "Ver notas", "Escrever uma nota", "Configurações", "Voltar", "Salvar",
    "Salvando…", "Dê um título à sua nota...", "Comece a escrever aqui...",
    "Um pensamento, só para você.", "Suas notas", "Escreva algo", "Idioma", "Aparência",
    "Modo escuro", "Mostrar datas", "Layout compacto", "Confirmar antes de excluir",
    "Tamanho do texto", "Ainda não há notas. Crie a primeira!",
    "Escreva algo antes de salvar.", "Nota salva.", "Configurações salvas.",
    "Não foi possível salvar a nota.", "Não foi possível abrir o banco de dados.",
    "Não foi possível ler as notas.", "Sem título", "Modificada", "Início", "Editar", "Excluir",
    "Copiar texto", "Copiado para a área de transferência.", "Excluir esta nota?", "Sim", "Não",
    "Nota excluída.", "Pesquisar notas...", "Nenhuma nota encontrada.", "Nota", "caracteres",
    "Não foi possível excluir a nota.",
];

const NL: Table = [
    "Nieuwe notitie", "Notities bekijken", "Notitie schrijven", "Instellingen", "Terug",
    "Opslaan", "Opslaan…", "Geef je notitie een titel...", "Begin hier met schrijven...",
    "Een gedachte, alleen voor jou.", "Je notities", "Schrijf iets", "Taal", "Weergave",
    "Donkere modus", "Tijdstempels tonen", "Compacte weergave", "Bevestigen voor verwijderen",
    "Tekstgrootte", "Nog geen notities. Maak je eerste!", "Schrijf iets voordat je opslaat.",
    "Notitie opgeslagen.", "Instellingen opgeslagen.", "Notitie kon niet worden opgeslagen.",
    "Database kon niet worden geopend.", "Notities konden niet worden gelezen.", "Zonder titel",
    "Gewijzigd", "Start", "Bewerken", "Verwijderen", "Tekst kopiëren",
    "Gekopieerd naar klembord.", "Deze notitie verwijderen?", "Ja", "Nee",
    "Notitie verwijderd.", "Notities zoeken...", "Geen notities gevonden.", "Notitie", "tekens",
    "Notitie kon niet worden verwijderd.",
];

const PL: Table = [
    "Nowa notatka", "Zobacz notatki", "Napisz notatkę", "Ustawienia", "Wstecz", "Zapisz",
    "Zapisywanie…", "Nadaj notatce tytuł...", "Zacznij pisać tutaj...",
    "Myśl tylko dla ciebie.", "Twoje notatki", "Napisz coś", "Język", "Wygląd", "Tryb ciemny",
    "Pokaż daty", "Układ kompaktowy", "Potwierdzaj przed usunięciem", "Rozmiar tekstu",
    "Brak notatek. Utwórz pierwszą!", "Napisz coś przed zapisaniem.", "Notatka zapisana.",
    "Ustawienia zapisane.", "Nie udało się zapisać notatki.", "Nie udało się otworzyć bazy danych.",
    "Nie udało się wczytać notatek.", "Bez tytułu", "Zmieniona", "Start", "Edytuj", "Usuń",
    "Kopiuj tekst", "Skopiowano do schowka.", "Usunąć tę notatkę?", "Tak", "Nie",
    "Notatka usunięta.", "Szukaj notatek...", "Brak pasujących notatek.", "Notatka", "znaków",
    "Nie udało się usunąć notatki.",
];

const RU: Table = [
    "Новая заметка", "Посмотреть заметки", "Написать заметку", "Настройки", "Назад",
    "Сохранить", "Сохранение…", "Дай заметке название...", "Начни писать здесь...",
    "Мысль только для тебя.", "Твои заметки", "Напиши что-нибудь", "Язык", "Оформление",
    "Тёмная тема", "Показывать дату", "Компактный вид", "Подтверждать удаление",
    "Размер текста", "Заметок пока нет. Создай первую!", "Напиши что-нибудь перед сохранением.",
    "Заметка сохранена.", "Настройки сохранены.", "Не удалось сохранить заметку.",
    "Не удалось открыть базу данных.", "Не удалось прочитать заметки.", "Без названия",
    "Изменено", "Главная", "Изменить", "Удалить", "Копировать текст",
    "Скопировано в буфер обмена.", "Удалить эту заметку?", "Да", "Нет", "Заметка удалена.",
    "Поиск заметок...", "Ничего не найдено.", "Заметка", "символов",
    "Не удалось удалить заметку.",
];

const UK: Table = [
    "Нова нотатка", "Переглянути нотатки", "Написати нотатку", "Налаштування", "Назад",
    "Зберегти", "Збереження…", "Дай нотатці назву...", "Почни писати тут...",
    "Думка лише для тебе.", "Твої нотатки", "Напиши щось", "Мова", "Вигляд", "Темна тема",
    "Показувати дату", "Компактний вигляд", "Підтверджувати видалення", "Розмір тексту",
    "Нотаток ще немає. Створи першу!", "Напиши щось перед збереженням.", "Нотатку збережено.",
    "Налаштування збережено.", "Не вдалося зберегти нотатку.", "Не вдалося відкрити базу даних.",
    "Не вдалося прочитати нотатки.", "Без назви", "Змінено", "Головна", "Редагувати",
    "Видалити", "Копіювати текст", "Скопійовано в буфер обміну.", "Видалити цю нотатку?",
    "Так", "Ні", "Нотатку видалено.", "Пошук нотаток...", "Нічого не знайдено.", "Нотатка",
    "символів", "Не вдалося видалити нотатку.",
];

const TR: Table = [
    "Yeni not", "Notları gör", "Not yaz", "Ayarlar", "Geri", "Kaydet", "Kaydediliyor…",
    "Notuna bir başlık ver...", "Burada yazmaya başla...", "Sadece senin için bir düşünce.",
    "Notların", "Bir şey yaz", "Dil", "Görünüm", "Karanlık mod", "Tarihleri göster",
    "Kompakt görünüm", "Silmeden önce onayla", "Yazı boyutu",
    "Henüz not yok. İlkini oluştur!", "Kaydetmeden önce bir şey yaz.", "Not kaydedildi.",
    "Ayarlar kaydedildi.", "Not kaydedilemedi.", "Veritabanı açılamadı.", "Notlar okunamadı.",
    "Başlıksız", "Değiştirildi", "Ana sayfa", "Düzenle", "Sil", "Metni kopyala",
    "Panoya kopyalandı.", "Bu not silinsin mi?", "Evet", "Hayır", "Not silindi.",
    "Notlarda ara...", "Eşleşen not yok.", "Not", "karakter", "Not silinemedi.",
];

const AR: Table = [
    "ملاحظة جديدة", "عرض الملاحظات", "اكتب ملاحظة", "الإعدادات", "رجوع", "حفظ", "جارٍ الحفظ…",
    "أعطِ ملاحظتك عنوانًا...", "ابدأ الكتابة هنا...", "فكرة لك وحدك.", "ملاحظاتك", "اكتب شيئًا",
    "اللغة", "المظهر", "الوضع الداكن", "إظهار التواريخ", "تخطيط مضغوط", "التأكيد قبل الحذف",
    "حجم الخط", "لا توجد ملاحظات بعد. أنشئ أول ملاحظة!", "اكتب شيئًا قبل الحفظ.",
    "تم حفظ الملاحظة.", "تم حفظ الإعدادات.", "تعذّر حفظ الملاحظة.",
    "تعذّر فتح قاعدة البيانات.", "تعذّرت قراءة الملاحظات.", "بلا عنوان", "عُدّلت", "الرئيسية",
    "تعديل", "حذف", "نسخ النص", "تم النسخ إلى الحافظة.", "حذف هذه الملاحظة؟", "نعم", "لا",
    "تم حذف الملاحظة.", "ابحث في الملاحظات...", "لا توجد ملاحظات مطابقة.", "ملاحظة", "حرف",
    "تعذّر حذف الملاحظة.",
];

const JA: Table = [
    "新しいノート", "ノートを見る", "ノートを書く", "設定", "戻る", "保存", "保存中…",
    "ノートにタイトルを付けよう...", "ここに書き始めよう...", "あなただけのひとこと。",
    "あなたのノート", "何か書く", "言語", "外観", "ダークモード", "日時を表示",
    "コンパクト表示", "削除前に確認", "文字サイズ", "ノートはまだありません。最初の1件を作ろう！",
    "保存する前に何か書いてください。", "ノートを保存しました。", "設定を保存しました。",
    "ノートを保存できませんでした。", "データベースを開けませんでした。",
    "ノートを読み込めませんでした。", "無題", "更新", "ホーム", "編集", "削除", "テキストをコピー",
    "クリップボードにコピーしました。", "このノートを削除しますか？", "はい", "いいえ",
    "ノートを削除しました。", "ノートを検索...", "一致するノートがありません。", "ノート", "文字",
    "ノートを削除できませんでした。",
];

const KO: Table = [
    "새 노트", "노트 보기", "노트 쓰기", "설정", "뒤로", "저장", "저장 중…",
    "노트에 제목을 지어 주세요...", "여기에 작성을 시작하세요...", "오직 나를 위한 생각 하나.",
    "내 노트", "무언가 쓰기", "언어", "화면 설정", "다크 모드", "날짜 표시", "간결한 레이아웃",
    "삭제 전 확인", "글자 크기", "아직 노트가 없습니다. 첫 노트를 만들어 보세요!",
    "저장하기 전에 내용을 입력하세요.", "노트가 저장되었습니다.", "설정이 저장되었습니다.",
    "노트를 저장할 수 없습니다.", "데이터베이스를 열 수 없습니다.", "노트를 읽을 수 없습니다.",
    "제목 없음", "수정됨", "홈", "수정", "삭제", "텍스트 복사", "클립보드에 복사되었습니다.",
    "이 노트를 삭제할까요?", "예", "아니요", "노트가 삭제되었습니다.", "노트 검색...",
    "일치하는 노트가 없습니다.", "노트", "자", "노트를 삭제할 수 없습니다.",
];

const ZH: Table = [
    "新建笔记", "查看笔记", "写笔记", "设置", "返回", "保存", "正在保存…", "给笔记起个标题...",
    "在这里开始写作...", "一个只属于你的想法。", "你的笔记", "写点什么", "语言", "外观",
    "深色模式", "显示时间", "紧凑布局", "删除前确认", "字体大小", "还没有笔记。创建第一条吧！",
    "保存前请先写点内容。", "笔记已保存。", "设置已保存。", "无法保存笔记。", "无法打开数据库。",
    "无法读取笔记。", "无标题", "已修改", "主页", "编辑", "删除", "复制文本", "已复制到剪贴板。",
    "删除这条笔记？", "是", "否", "笔记已删除。", "搜索笔记...", "没有匹配的笔记。", "笔记", "字符",
    "无法删除笔记。",
];

const HI: Table = [
    "नया नोट", "नोट देखें", "नोट लिखें", "सेटिंग्स", "वापस", "सहेजें", "सहेज रहे हैं…",
    "अपने नोट को एक शीर्षक दें...", "यहाँ लिखना शुरू करें...", "एक विचार, सिर्फ़ आपके लिए।",
    "आपके नोट", "कुछ लिखें", "भाषा", "रूप-रंग", "डार्क मोड", "तारीख़ दिखाएँ", "सघन लेआउट",
    "हटाने से पहले पुष्टि करें", "अक्षर का आकार", "अभी कोई नोट नहीं है। पहला नोट बनाएँ!",
    "सहेजने से पहले कुछ लिखें।", "नोट सहेज लिया गया।", "सेटिंग्स सहेज ली गईं।",
    "नोट सहेजा नहीं जा सका।", "डेटाबेस नहीं खुल सका।", "नोट पढ़े नहीं जा सके।", "शीर्षकहीन",
    "बदला गया", "होम", "संपादित करें", "हटाएँ", "टेक्स्ट कॉपी करें", "क्लिपबोर्ड पर कॉपी हो गया।",
    "यह नोट हटाएँ?", "हाँ", "नहीं", "नोट हटा दिया गया।", "नोट खोजें...", "कोई मेल खाता नोट नहीं।",
    "नोट", "अक्षर", "नोट हटाया नहीं जा सका।",
];

const SV: Table = [
    "Ny anteckning", "Visa anteckningar", "Skriv en anteckning", "Inställningar", "Tillbaka",
    "Spara", "Sparar…", "Ge din anteckning en titel...", "Börja skriva här...",
    "En tanke, bara för dig.", "Dina anteckningar", "Skriv något", "Språk", "Utseende",
    "Mörkt läge", "Visa tidsstämplar", "Kompakt layout", "Bekräfta före borttagning",
    "Textstorlek", "Inga anteckningar än. Skapa din första!", "Skriv något innan du sparar.",
    "Anteckningen sparades.", "Inställningarna sparades.", "Kunde inte spara anteckningen.",
    "Kunde inte öppna databasen.", "Kunde inte läsa anteckningarna.", "Utan titel", "Ändrad",
    "Hem", "Redigera", "Ta bort", "Kopiera text", "Kopierat till urklipp.",
    "Ta bort den här anteckningen?", "Ja", "Nej", "Anteckningen togs bort.",
    "Sök anteckningar...", "Inga matchande anteckningar.", "Anteckning", "tecken",
    "Kunde inte ta bort anteckningen.",
];

const EL: Table = [
    "Νέα σημείωση", "Προβολή σημειώσεων", "Γράψε μια σημείωση", "Ρυθμίσεις", "Πίσω",
    "Αποθήκευση", "Αποθήκευση…", "Δώσε έναν τίτλο στη σημείωση...", "Ξεκίνα να γράφεις εδώ...",
    "Μια σκέψη, μόνο για σένα.", "Οι σημειώσεις σου", "Γράψε κάτι", "Γλώσσα", "Εμφάνιση",
    "Σκοτεινή λειτουργία", "Εμφάνιση ημερομηνιών", "Συμπαγής διάταξη",
    "Επιβεβαίωση πριν τη διαγραφή", "Μέγεθος κειμένου",
    "Δεν υπάρχουν σημειώσεις ακόμα. Δημιούργησε την πρώτη!", "Γράψε κάτι πριν αποθηκεύσεις.",
    "Η σημείωση αποθηκεύτηκε.", "Οι ρυθμίσεις αποθηκεύτηκαν.",
    "Δεν ήταν δυνατή η αποθήκευση της σημείωσης.", "Δεν ήταν δυνατό το άνοιγμα της βάσης.",
    "Δεν ήταν δυνατή η ανάγνωση των σημειώσεων.", "Χωρίς τίτλο", "Τροποποιήθηκε", "Αρχική",
    "Επεξεργασία", "Διαγραφή", "Αντιγραφή κειμένου", "Αντιγράφηκε στο πρόχειρο.",
    "Διαγραφή αυτής της σημείωσης;", "Ναι", "Όχι", "Η σημείωση διαγράφηκε.",
    "Αναζήτηση σημειώσεων...", "Καμία αντίστοιχη σημείωση.", "Σημείωση", "χαρακτήρες",
    "Δεν ήταν δυνατή η διαγραφή της σημείωσης.",
];

const RO: Table = [
    "Notă nouă", "Vezi notele", "Scrie o notă", "Setări", "Înapoi", "Salvează", "Se salvează…",
    "Dă un titlu notei tale...", "Începe să scrii aici...", "Un gând, doar pentru tine.",
    "Notele tale", "Scrie ceva", "Limbă", "Aspect", "Mod întunecat", "Arată datele",
    "Aspect compact", "Confirmă înainte de ștergere", "Dimensiunea textului",
    "Nicio notă încă. Creează-o pe prima!", "Scrie ceva înainte de a salva.", "Notă salvată.",
    "Setări salvate.", "Nota nu a putut fi salvată.", "Baza de date nu a putut fi deschisă.",
    "Notele nu au putut fi citite.", "Fără titlu", "Modificată", "Acasă", "Editează", "Șterge",
    "Copiază textul", "Copiat în clipboard.", "Ștergi această notă?", "Da", "Nu",
    "Notă ștearsă.", "Caută în note...", "Nicio notă găsită.", "Notă", "caractere",
    "Nota nu a putut fi ștearsă.",
];

const CS: Table = [
    "Nová poznámka", "Zobrazit poznámky", "Napsat poznámku", "Nastavení", "Zpět", "Uložit",
    "Ukládání…", "Dej poznámce název...", "Začni psát tady...", "Myšlenka jen pro tebe.",
    "Tvé poznámky", "Napiš něco", "Jazyk", "Vzhled", "Tmavý režim", "Zobrazit data",
    "Kompaktní rozvržení", "Potvrdit před smazáním", "Velikost textu",
    "Zatím žádné poznámky. Vytvoř první!", "Před uložením něco napiš.", "Poznámka uložena.",
    "Nastavení uloženo.", "Poznámku se nepodařilo uložit.", "Databázi se nepodařilo otevřít.",
    "Poznámky se nepodařilo načíst.", "Bez názvu", "Upraveno", "Domů", "Upravit", "Smazat",
    "Kopírovat text", "Zkopírováno do schránky.", "Smazat tuto poznámku?", "Ano", "Ne",
    "Poznámka smazána.", "Hledat poznámky...", "Žádné odpovídající poznámky.", "Poznámka",
    "znaků", "Poznámku se nepodařilo smazat.",
];

fn table(language: Language) -> &'static Table {
    match language {
        Language::English => &EN,
        Language::French => &FR,
        Language::Spanish => &ES,
        Language::German => &DE,
        Language::Italian => &IT,
        Language::Portuguese => &PT,
        Language::Dutch => &NL,
        Language::Polish => &PL,
        Language::Russian => &RU,
        Language::Ukrainian => &UK,
        Language::Turkish => &TR,
        Language::Arabic => &AR,
        Language::Japanese => &JA,
        Language::Korean => &KO,
        Language::Chinese => &ZH,
        Language::Hindi => &HI,
        Language::Swedish => &SV,
        Language::Greek => &EL,
        Language::Romanian => &RO,
        Language::Czech => &CS,
    }
}

/// Texte traduit pour `key` ; renvoie la clé elle-même si elle est inconnue.
pub fn tr(language: Language, key: &str) -> &str {
    match KEYS.iter().position(|k| *k == key) {
        Some(index) => table(language)[index],
        None => key,
    }
}
