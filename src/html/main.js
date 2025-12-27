function main() {
   // main app functionality
    document.addEventListener('keydown', e => {
      if (e.ctrlKey && (e.key === 's' || e.key === 'S')) {
        e.preventDefault()
        saveData(document.querySelector('input[name="tabs"]:checked').id)
      }
      if (e.ctrlKey && e.code == 'KeyB') {
        e.preventDefault()
        toggleBookmark()
      }
    })
    
    window.onbeforeunload = function(e) {
        var ev = e || window.event;
        autosave()
        if (STORE_TABS)
            storeTabs()
        storeBookmarks()
        if (AUTOSAVE)
            return false
        e.preventDefault(); // trigger to show confirmation dialof
        const msg = 'Make sure to save your work before reloading the page or to navigate out'
        // For IE and Firefox
        if (ev) {
            ev.returnValue = msg
        }
        return msg
    }
    
    if(document.addEventListener) document.addEventListener("visibilitychange", autosave)
    
    document.getElementById('commandarea').addEventListener("focus", (event) => { autosave() }) // can be an overhead and a race condition
    
    document.addEventListener('copy', function(event) {
        if (!document.querySelector('#cpySpec').classList.contains('marked'))
            return
        // Get the selected text
        let selectedText = window.getSelection().toString();
        
        // Modify the text
        const lines = selectedText.split(/\r?\n/);
        let modifiedText = ''
        for (const line of lines) {
            const len = line.length
            var i=0
            var frstBl = true
            for (; i<len; ++i) {
                const char = line.charAt(i)
                if (char >= '0' && char <= '9' || char == '+' || char == '-' || char == '~') {
                    continue
                }
                if (frstBl && char == ' ') {
                    frstBl = false
                    continue
                }
                break
            }
            if (i > 0) {
                if (len > i && line.charAt(i) == ' ')
                    i++
                modifiedText += line.slice(i)
            } else {
                modifiedText += line
            }
            modifiedText += '\n'
        }
        
        // Set the modified data to the clipboard
        event.clipboardData.setData('text/plain', modifiedText);
        
        // Prevent the default copy action
        event.preventDefault(); 
        
        //console.log("Copied modified text via event listener: " + modifiedText);
    })
        
    if (UID) {
        wsSession = 'webId-' + UID
    } else {
        wsSession = 'webId-' + makeid(12) // only allowed chars in HTTP header
    }
    
    ace.require("ace/ext/language_tools");
    
    loadSettings()
    
    vcsStatus()
    
    loadBookmarks()
}

function getVersion() {
    return '1.10.02.103'
}

function populateProjectTree() {
     // populate file tree
    ajax.get({url:"./rustcgi?mode=tree&session="+encodeURIComponent(SESSION), success: render_tree})
}

function render_tree(tree) {
    const dw = document.querySelector('.left-tree')
    dw.innerHTML = ''
     // reset lock
    lockLoad = false
    renderTree([tree], dw, true)
}

function loadSettings() {
    ajax.get({url:"./rustcgi?mode=settings-project&session="+encodeURIComponent(SESSION), success: function(load) {
             // document.getElementById("place").value = load.project_home
              HOME_LEN = load.home_len
              if (load && load.theme)
                  THEME = load.theme
              if (load && load.autosave)
                  AUTOSAVE = 'yes' == load.autosave
              else
                  AUTOSAVE = false
              if (load && load.persist_tabs)
                  STORE_TABS = 'yes' == load.persist_tabs
              else
                  STORE_TABS = false
              if (load && load.project_home)
                   PROJECT_HOME = load.project_home
              if (load && load.ai_server_url)
                   AI_SERVER_URL = load.ai_server_url
              if (load && load.project_config) {
                  PROJ_CONF = JSON.parse(load.project_config);
              }
              if (load && load.colapsed_dirs)
                  COLAPSED_DIRS = load.colapsed_dirs
              if (load && load.src_dir)
                  SRC_DIR= load.src_dir
              // TODO other inits
              populateProjectTree()
              if (STORE_TABS)
                  loadTabs()
              loadNotepad() 
              if (load && load.proj_conf)
                PROJ_CONF = load.proj_conf
              loadProjConf()
              ws_setup()
              ws_term_connect()
              if (!load.project_home)
                newProject()
          }, fail: function(cc, err) {
              ws_setup()
              ws_term_connect()
          }})
}

function render_editor(edittab, path) {
    const tabs = document.querySelector('.tabs')
    if (tabs.children.length == 0)
       tabs.innerHTML = edittab
    else {
        tabs.lastElementChild.insertAdjacentHTML(
            'afterend', edittab)
    }
    var editor = ace.edit("editor"+path)
    if (THEME == '')
        THEME = 'light'
    editor.setTheme("ace/theme/"+EDITOR_THEME[THEME])
    const ext = path.split('.').pop()
    if (EDITOR_MODE[ext])
        editor.session.setMode("ace/mode/"+EDITOR_MODE[ext])
    editor.setAutoScrollEditorIntoView(false)
    if (ext == 'txt' || ext == 'md')
        editor.setOption("spellcheck", true)
    //editor.setFontSize(16)
    editor.setOptions({
        fontSize: "12pt",
        enableBasicAutocompletion: true, // Enables basic autocompletion
        enableSnippets: true,            // Enables code snippets
        enableLiveAutocompletion: true
    })
    editor.resize()
    const editorArea = document.querySelector('div.center-pane');
    const viewportHeight = editorArea.getBoundingClientRect().height
    var lines = viewportHeight / editor.renderer.lineHeight
    if (lines > 4) {
        lines = lines - 2
    } else if (lines == 0) {
        lines = 3
    }
    editor.setOption('maxLines', lines) // was Infinity)

    const data = editor.getValue()
    const dataSize = data.length
    const crc = crc32(data)
    //console.log("crc:"+crc)
    EDITORS[path] = {'editor':editor, 'size':dataSize, 'crc':crc}
}

function render_editor_js(json) {
    const tab = `<input type="radio" name="tabs" id="${htmlAttrEncode(json.path)}" checked="checked" data-modified="${json.modified}">
      <label for="${htmlAttrEncode(json.path)}" title="${htmlAttrEncode(json.path)}">${htmlEncode(json.name)}</label>
      <div class="tab">
         <pre id="editor${htmlAttrEncode(json.path)}">
${htmlEncode(json.content)}</pre>
      </div>`
    render_editor(tab, json.path)
}

var lockLoad

function renderTree(data, parentElement, rootEl) {
  const ul = document.createElement('ul');
  parentElement.appendChild(ul);
  data.forEach((item) => {
      if (item.type == 'dead')
        return
    const li = document.createElement('li');
    const span = document.createElement('span');
    span.textContent = item.name;
    span.className = item.type;
    if (item.modified) {
        span.title = new Date(item.modified * 1000).toLocaleString()
    }
    li.appendChild(span);
    if (rootEl) {
        const minSpan = document.createElement('span')
        minSpan.textContent = '🗕' 
        minSpan.style = 'float:right;cursor:pointer;'
        // add click toggleMinimize()
        minSpan.addEventListener("click", () => {
            const tree = document.querySelector('.left-tree')
            if (tree.style.maxWidth != '') {
                tree.style.maxWidth = ''
                minSpan.textContent = '🗕'
            } else {
                tree.style.maxWidth = '4em'
                minSpan.textContent = '⛶'
            }
        })
        li.appendChild(minSpan)
    }
    
    ul.appendChild(li);
    
    var path = [item.name]
    var pe = li.parentElement
    while (pe && (pe.tagName == 'UL' || pe.tagName == 'LI')) {
        if (pe.tagName == 'LI') 
           path.push(pe.firstChild.innerHTML)
        pe = pe.parentElement
    }
    path.pop()
    //path = decodeHtmlEntity(path)
    const pathstr = path.reverse().join('/')
    span.onclick = //li.addEventListener('click',
    function () {
        // possibly separate a file and a folder handlers
        if (item.type == 'file') {
            //alert(pathstr+' for '+item.name)
            const tab_item = document.getElementById(pathstr)
            if (tab_item) {
                tab_item.checked = true
            } else if (!lockLoad) {
                lockLoad = true
               ajax.get({url:"./rustcgi?mode=editor-file&name="+encodeURIComponent(decodeHtmlEntity(item.name))+"&path="+encodeURIComponent(decodeHtmlEntity(pathstr))+
                 "&session="+encodeURIComponent(SESSION), success: function (json) { lockLoad = false; render_editor_js(json)},
                 fail: function(ecode,etext) {lockLoad = false;showErrorMessage(`File ${pathstr} can't be load /${etext}`)}})
            }
        } else if (item.type == 'folder') {
            for (var el of li.children) {
                if (el.tagName == 'UL') {
                    if (el.style.display == 'none')
                        el.style.display = 'block'
                    else
                       el.style.display = 'none'
                    break
                }
            }
          }
    }//)

    if (item.children && item.children.length > 0) {
        renderTree(item.children, li);
        if (COLAPSED_DIRS.includes(item.name) && item.type == 'folder') {
          for (child of li.children) {
              if (child.tagName == 'UL') {
                    child.style.display = 'none'
                    break
              }
          }
        }
    }
  });
}

function moveToLineInFile(path, line, col, failHandler) {
    //console.log('path to show '+path+' at '+line)
    const tab_item = document.getElementById(path)
    
    // TODO make it a function
    if (tab_item) {
        tab_item.checked = true
        EDITORS[path].editor.gotoLine(line, col, true)
    } else if (!lockLoad) {
        lockLoad = true
        const name = path.split('/').pop()
        ajax.get({url:"./rustcgi?mode=editor-file&name="+encodeURIComponent(name)+"&path="+encodeURIComponent(path)+
             "&session="+encodeURIComponent(SESSION), success: function (json) { lockLoad = false; render_editor_js(json)
                 EDITORS[path].editor.gotoLine(line, col, true)
             },
             fail: function(ecode, etext) {lockLoad = false; if (failHandler && failHandler instanceof Function) failHandler(path, line, col)}})
    }
}

function loadFileStack(stack) {
    if (stack.length == 0) {
        lockLoad = false
        return
    }
    var path = stack.pop()
    const colSep = path.lastIndexOf(':')
    var line = 1
    var col = 1
    if (colSep > 0) {
        // a potential problem if a path looks like dir1/dir2/somechars:somenum1:somenum2 and position info just omitted
        col = Number(path.substring(colSep+1))
        path = path.substring(0, colSep)
        const lineSep = path.lastIndexOf(':')
        if (lineSep > 0) {
            line = Number(path.substring(lineSep+1))
            path = path.substring(0, lineSep)
        }
    }
    const name = path.split('/').pop()
    ajax.get({url:"./rustcgi?mode=editor-file&name="+encodeURIComponent(name)+"&path="+encodeURIComponent(path)+
         "&session="+encodeURIComponent(SESSION), success: function (json) { render_editor_js(json)
             EDITORS[path].editor.gotoLine(line, col, true)
             loadFileStack(stack)
         },
         fail: function(ecode, etext) { }})
}

function saveData(path, newpath, onsave) {
    const data = EDITORS[path]['editor'].getValue()
    const dataSize = data.length
    const crc = crc32(data)
    if (newpath)
        path = newpath
    else { // check if file changed
        if (dataSize == EDITORS[path]['size']) {
            if (EDITORS[path]['crc'] == crc) {
                console.log('file '+path+' is the same')
                return false
            }
        }
    }
    const docTab =    document.getElementById(path)
    const modified = docTab? docTab.dataset.modified:0
    const payload = 'mode=save&name='+encodeURIComponent(path)+'&data='+encodeURIComponent(data)+'&modified='+modified
        +"&session="+encodeURIComponent(SESSION)
    ajax.post({url:"./rustcgi", query: payload,
        success: function (resp) { 
          if (resp && !resp.startsWith('Ok')) {
              showErrorMessage('File '+path+' can\'t be saved, (' + resp + ')')
          	  EDITORS[path]['editor'].setReadOnly(true)
          } else {
             if (resp.startsWith('Ok')) {
                 if (docTab)
                     docTab.dataset.modified = resp.substring(3)
                 if (onsave && typeof onsave === "function")
                     onsave()
                 if (EDITORS[path]) {
                     EDITORS[path]['size'] = dataSize
                     EDITORS[path]['crc'] = crc
                 } else
                    console.log('an editor for ' + path + ' isn\'t set yet')
             } else 
                showErrorMessage('File '+path+' can\'t be saved, see log for details: '+ resp)
          }
        },
        fail: function(code, reason) {
            showErrorMessage('File '+path+' can\'t be saved, see log for details: '+ reason)
        }, respType:"html"})
    return true
}

function newFileTab(name, path) {
    const tab = '<input type="radio" name="tabs" id="'+htmlAttrEncode(path)+'" checked="checked" data-modified="0">' +
    '<label for="'+htmlAttrEncode(path)+'" title="'+htmlAttrEncode(path)+'">'+htmlEncode(name)+'</label>' +
   ' <div class="tab">' +
      '<pre id="editor'+htmlAttrEncode(path)+'">new</pre>' +
    '</div>'
    render_editor(tab, path)
}

function storeBookmarks() {
    let res = []
    const bookmarks = document.querySelector('#bookmarks')
    for (const child of bookmarks.children) {
        res.push({src:child.dataset.src,line:child.dataset.line,content:child.dataset.snip,comment:child.dataset.note})
    }
    ajax.post({url:"./rustcgi", query: 'mode=save-bookmark&session='+encodeURIComponent(SESSION)+'&bookmarks='+encodeURIComponent(JSON.stringify(res)),
        success: function (data) { 
                 // no check sucess since at exit
             }, respType:"html"})
}

function loadBookmarks() {
     ajax.get({url:"./rustcgi?mode=load-bookmark&session="+encodeURIComponent(SESSION), success: function(bookmarks) {
            const bookmarkList = document.querySelector('#bookmarks')
            bookmarks.sort((a, b) => a.src == b.src?a.line - b.line:a.src > b.src?1:-1 );
            for (const bookmark of bookmarks) {
                const bookmarkEl = document.createElement("LI");
                bookmarkEl.dataset.line = bookmark.line
                bookmarkEl.dataset.src = bookmark.src
                bookmarkEl.dataset.snip = bookmark.content
                bookmarkEl.dataset.note = bookmark.comment
                bookmarkEl.textContent = bookmark.src.split('/').pop() + ':' + bookmark.line + ' - ' + trimAndEllipsize(bookmark.content, 36)
                bookmarkEl.addEventListener("click", () => {
                    moveToLineInFile(bookmarkEl.dataset.src, bookmarkEl.dataset.line, 0, clearBookmark)
                });
                bookmarkList.appendChild(bookmarkEl)
            }
        }
    })
}

function clearBookmark(src, line, col) {
    const bookmarks = document.querySelector('#bookmarks')
    for (const child of bookmarks.children) {
        if (child.dataset.src == src && child.dataset.line == line) {
            bookmarks.removeChild(child)
            break
        }
    }
}

function loadNotepad() {
    //document.getElementById("projectnp").checked
    ajax.get({url:"./rustcgi?mode=loadnp&session=" + encodeURIComponent(SESSION), success: function(load) {
        document.getElementById("notepad").value = load 
    }, respType:"html"})  
}

function saveAllFiles() {
    let saved = false
    for(var key in EDITORS){
        if (key.indexOf('\t') < 0)
            saved |= saveData(key)
    }
    return saved
}

function autosave() {
    if (AUTOSAVE)
    // scan all opened editors
         Object.keys(EDITORS).forEach(key => saveData(key))
}

function loadTabs() {
    // assumes there is no opened tab
    if (EDITORS.size > 0)
        return
    ajax.get({url:"./rustcgi?mode=load-persist-tab&session="+encodeURIComponent(SESSION), success: function(load) {
        lockLoad =  true
        loadFileStack(load)
    }
    })
}

function storeTabs() {
    var tabPaths = ''
    for(var key in EDITORS){
        const currCur = EDITORS[key].editor.getCursorPosition()
        tabPaths += '\t' + key + ':' + currCur.row  + ':' + currCur.column
    }
    ajax.post({url:"./rustcgi", query: "mode=persist-tab&session="+encodeURIComponent(SESSION) +
        '&tabs='+encodeURIComponent(tabPaths) , success: function (data) { 
         if (!data.startsWith('Ok')) {
            showErrorMessage('An error occurred : ' + data)
         }
    }, respType:"html"})
}

function closeEditor(path) {
    delete  EDITORS[path]
    const r = document.getElementById(path)
    var removing_elements = [r]
    var next = r.nextSibling
    while (next.tagName != 'DIV') {
        removing_elements.push(next)
        next = next.nextSibling
    }
    next.remove()
    next = removing_elements.pop()
    while (next) {
       next.remove()
       next = removing_elements.pop()
    }
    // select some existing tab if any
}

function closeAll() {
    const tabs = document.querySelector('.tabs')
    if (tabs.children.length > 0)
       tabs.innerHTML =   ''
    for (var key in EDITORS) {
       delete EDITORS[key];
    }
}

function showErrorMessage(msg) {
    // show animated fixed position message line
    const bar = document.querySelector('.message-bar')
    bar.innerHTML = '<p>'+htmlEncode(msg)+'</p>'
    bar.style.display = 'block'
    //alert('Error:' + msg)
}

// terminal specific
    var WS_TERM_URL
function extendURL(lineStr) {
    const urlRegex = /https?:\/\/[a-zA-Z0-9-._~/?#]+/g;
    return lineStr.replace(urlRegex, (url) => {
              return '<a href="' + url + '" target="_blank">' + url + '</a>';
        }).replace(fileNameReg, (fileName) => {
        const matches = [...fileName.matchAll(fileNameReg)];
        if (matches.length > 0) {
            const file = matches[0].groups.file;
            const line = matches[0].groups.line;
            const col = matches[0].groups.col;
            var path = matches[0].groups.path
            if (path.startsWith('/') || path.indexOf(':\\') == 1) // current OS root
                path = path.substring(HOME_LEN + PROJECT_HOME.length+1)
            path = path.replaceAll('\\', '/')
            const extraPath = SRC_DIR==''?'':SRC_DIR + '/' //'src/'
            return `<a href="javascript:moveToLineInFile('${path}${extraPath}${file}',${line},${col})">${fileName}</a>`
        } else {
            return fileName // dead code
        }
    })
}
    
function ws_setup() {
    const sec = window.location.protocol === 'https:'?'s':''
    const port = location.port // 8443 for brutal use different server
    const portExt = (sec == '' && port != 80 && port != '' || sec == 's' && port != 443 && port != '')?':'+port:''
    // PROJECT_HOME
    var project = SESSION
    if (!SESSION || SESSION == '')
        project = 'default'
    else
      project = SESSION
    WS_TERM_URL = 
       //'ws'+sec+'://'+location.hostname+portExt+
       '/terminal/'+ encodeURIComponent(project) + '/' + wsSession + '?version=' + getVersion() 
      // + '&project=' + encodeURIComponent(SESSION)
    //console.log('ws url: '+ws_url)
}

// //////  Util methods
function getSelectionText(everywhere) {
    if (everywhere) {
        var text = "";
        if (window.getSelection) {
            text = window.getSelection().toString();
        } else if (document.selection && document.selection.type != "Control") {
            text = document.selection.createRange().text;
        }
        return text;
    } else {
        const editor_tab = document.querySelector('input[name="tabs"]:checked')
        if (editor_tab) {
            const editor = EDITORS[editor_tab.id].editor
            return editor.getSelectedText()
        }
    }
    return ''
}

function removeOptions(selectElement) {
   var i, L = selectElement.options.length - 1;
   for(i = L; i >= 0; i--) {
      selectElement.remove(i);
   }
}

var makeCRCTable = function(){
    var c;
    var crcTable = [];
    for(var n =0; n < 256; n++){
        c = n;
        for(var k =0; k < 8; k++){
            c = ((c&1) ? (0xEDB88320 ^ (c >>> 1)) : (c >>> 1));
        }
        crcTable[n] = c;
    }
    return crcTable;
}

var crc32 = function(str) {
    var crcTable = window.crcTable || (window.crcTable = makeCRCTable());
    var crc = 0 ^ (-1);

    for (var i = 0; i < str.length; i++ ) {
        crc = (crc >>> 8) ^ crcTable[(crc ^ str.charCodeAt(i)) & 0xFF];
    }

    return (crc ^ (-1)) >>> 0;
};