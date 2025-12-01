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
    return '1.10.00.095'
}

function populateProjectTree() {
     // populate file tree
    ajax.get({url:"./rustcgi?mode=tree&session="+encodeURIComponent(SESSION), success: render_tree, respType:"json"})
}

function render_tree(tree) {
    const dw = document.querySelector('.left-tree')
    dw.innerHTML = ''
     // reset lock
    lockLoad = false
    renderTree([tree], dw)
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
              ws_connect()
              if (!load.project_home)
                newProject()
          }, fail: function(cc, err) {
              ws_setup()
              ws_connect()
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
    const lines = viewportHeight / editor.renderer.lineHeight

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
         <pre id="editor${htmlAttrEncode(json.path)}">${htmlEncode(json.content)}</pre>
      </div>`
    render_editor(tab, json.path)
}

var lockLoad

function renderTree(data, parentElement) {
  const ul = document.createElement('ul');
  parentElement.appendChild(ul);

  data.forEach((item) => {
      if (item.type == 'dead')
        return
    const li = document.createElement('li');
    const span = document.createElement('span');
    span.textContent = item.name;
    span.className = item.type;
    li.appendChild(span);
    
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
                 fail: function(ecode,etext) {lockLoad = false;showErrorMessage(`File ${pathstr} can't be load currently`)}})
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
    var notifRecon = 0
    var maxReconn = 16 * 1000
    var ws_url
    const commandBuffer = []
    var cmdBufPos = -1
    
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
    ws_url = 
       //'ws'+sec+'://'+location.hostname+portExt+
       '/terminal/'+ encodeURIComponent(project) + '/' + wsSession + '?version=' + getVersion() 
      // + '&project=' + encodeURIComponent(SESSION)
    //console.log('ws url: '+ws_url)
}

// source https://i.sstatic.net/9UVnC.png
const PAL_LOOKUP = ["rgb(12,12,12)", "rgb(197,15,31)", "rgb(19,161,14)", "rgb(193,156,0)", "rgb(0, 71, 171)",
                    "rgb(136,23,152)", "rgb(58,150,221)", "rgb(204,204,204)", "rgb(118,118,118)", "rgb(231,72,86)",
                    "rgb(22,198,12)", "rgb(246,234,158)", "rgb(59,120,255)", "rgb(180,0,158)", "rgb(97,214,214)", "rgb(242,242,242)"]
// "rgb(250,229,135)"
var lastChunk = ''

/* */
var bold = false
var under = false
var revs = false
var strike = false
var italic = false
var hide = false
var blink = false
var fon_color = ''
var fon_back = ''
/*  */            
            
function clearColorAttributes() {
    /* */
    bold = false
    under = false
    revs = false
    strike = false
    italic = false
    hide = false
    blink = false
    fon_color = ''
    fon_back = ''
     /*                   */
}

function ws_connect() {
    wskt = new WebSocket(ws_url)
    wskt.onopen = function(d) {
         notifRecon = 500
     }
     wskt.onmessage = function(e) {
         
        if (e.data.startsWith('\r') && (e.data.length == 1 || e.data.charAt(1) != '\n')) { // not very relaible now since can interfere with a regular out
            const cmd = document.getElementById('commandarea')
            var cmdStr
            if (e.data.slice(-1) == '\x07') {
                cmdStr = e.data.slice(1, -1).trim();
                beep()
            } else {
                cmdStr = e.data.trim()
            }
            if (cmdStr)
                    cmd.innerHTML = htmlEncode(cmdStr)
            cmd.focus()
            document.execCommand('selectAll', false, null)
            document.getSelection().collapseToEnd()
            return
        }
        
        const cons = document.querySelector('#terminal')
        var noPrompt = true
        
        //console.log(e.data)  // for debug
        // the code handles the situation when data split between two chunks (not more than two though)
        var chunk = e.data
        if (chunk.charAt(chunk.length - 1) == '\f') {
            noPrompt = false
            chunk = chunk.slice(0, -1)
        }
  
        if (chunk.charAt(chunk.length - 1) == '\x1b') {
             chunk = lastChunk + chunk.substring(0, chunk.length-1)
             lastChunk = '\x1b'
        } else if (chunk.length >= 2 && chunk.charAt(chunk.length - 2) == '\x1b' && chunk.charAt(chunk.length - 1) == '[') {
             chunk = lastChunk + chunk.substring(0, chunk.length-2)
             lastChunk = '\x1b['
        } else {
            const lastEsc = chunk.lastIndexOf('\x1b[')
            if (lastEsc > 0 && chunk.indexOf('m', lastEsc) < 0) {
                const over = chunk.substring(lastEsc)
                chunk = lastChunk + chunk.substring(0, lastEsc)
                lastChunk = over
            } else {
                if (lastChunk) {
                    chunk = lastChunk + chunk
                    lastChunk = ''
                }
            }
        }
        if ( chunk . startsWith('\x1b[') && chunk.indexOf('m') < 0) {
            lastChunk = chunk
            return
        }
        var wasEsc = chunk . startsWith('\x1b[')
        const ansi_esc = chunk.split(/\x1b\[/g)
        const term_frag = document.createElement("pre")
        if (ansi_esc.length > 1) {
            var ansi_html = ''
            // assure esc[0m when stream closed on the endpoint side
            var shift
            for (var ans of ansi_esc) {
                // procceed ANSI code
                shift = 0
                //if (false && wasEsc) {
                if (wasEsc) {
                do {
                    if (ans.charAt(shift) == '0' || ans.charAt(shift) == 'm') { // reset
                        clearColorAttributes()
                        if (ans.charAt(shift) != 'm')
                           shift ++
                    } else if (ans.charAt(shift) == '9') {
                        if (ans.charAt(shift+1) >= '0' && ans.charAt(shift+1) <= '9') {
                            fon_color = PAL_LOOKUP[Number(ans.charAt(shift + 1)) + 8]
                            shift += 2
                        } else { // ; or m
                            strike = true
                            shift++
                        }
                    } else if (ans.charAt(shift) == '4' && ans.charAt(shift + 1) != ';' && ans.charAt(shift + 1) != 'm' && ans.charAt(shift + 1) != '8'  && ans.charAt(shift + 1) != '9') {
                        fon_back = PAL_LOOKUP[ans.charAt(shift + 1)]
                        shift += 2
                    } else if ( ans.charAt(shift) == '1' && ans.charAt(shift + 1) == '0') {
                        fon_back = PAL_LOOKUP[Number(ans.charAt(shift + 2)) + 8]
                        shift += 3
                    }  else if (ans.charAt(shift) == '1') {
                        bold = true
                        shift += 1
                    } else if (ans.charAt(shift) == '7') { // investigate how manage dark theme
                        fon_color = 'Canvas'
                        fon_back = 'CanvasText'
                        shift += 1
                    } else if (ans.charAt(shift) == '2' && ans.charAt(shift+1) == '4') {
                        under = false
                        shift += 2
                    } else if (ans.charAt(shift) == '2' && ans.charAt(shift+1) == '7') {
                        fon_back = 'Canvas'
                        fon_color = 'CanvasText'
                        shift += 2
                    } else if (( ans.charAt(shift) == '3' || ans.charAt(shift) == '4') && (ans.length-shift) > 5 && ans.charAt(shift + 1) == '8') {
                        if (ans.charAt(shift + 2) == ';' && ans.charAt(shift + 3) == '5' &&
                            ans.charAt(shift + 4) == ';') {
                            const forg = ans.charAt(shift) == '3'
                            shift += 5
                            // find 'm' and get number
                            var colNum 
                            if (ans.charAt(shift + 1) == 'm') {
                                colNum = ans.charAt(shift)
                                shift += 1
                            } else if (ans.charAt(shift + 2) == 'm') {
                                colNum = Number(ans.substring(shift, shift+2))
                                shift += 2
                            }
                            if (colNum > -1 && colNum < PAL_LOOKUP.length) {
                                if (!(WIN_SERVER && colNum == PAL_LOOKUP.length - 1)) {
                                    if (forg)
                                        fon_color = PAL_LOOKUP[colNum]
                                    else
                                        fon_back = PAL_LOOKUP[colNum]
                                }
                            } else
                                shift = 0
                        } else
                            shift = 0
                    } else if (ans.charAt(shift) == '4') {
                        if (ans.charAt(shift+1) >= '0' && ans.charAt(shift+1) <= '9') {
                            if (ans.charAt(shift+1) == '9') {
                                fon_back = ''
                            }
                            shift += 2
                        } else {
                            under = true
                            shift += 1
                        }
                    } else if (ans.charAt(shift) == '3') {
                        if (ans.charAt(shift + 1) >= '0' && ans.charAt(shift + 1) <= '7') {
                            fon_color = PAL_LOOKUP[ans.charAt(shift + 1)]
                            shift += 2
                        } else if (ans.charAt(shift + 1) == '9') {
                            fon_color = '' 
                            shift += 2
                        } else {
                            italic = true
                            shift += 1
                        }
                    } else if (ans.charAt(shift) == '5' || ans.charAt(shift) == '6') {
                        if (ans.charAt(shift+1) >= '0' && ans.charAt(shift+1) <= '9') {
                            shift += 2
                        } else {
                            blink = true
                            shift += 1
                        }
                    } else if (ans.charAt(shift) == '8') {
                        if (ans.charAt(shift+1) >= '0' && ans.charAt(shift+1) <= '9') {
                            shift += 2
                        } else {
                            hide = true
                            shift += 1
                        }
                    } else 
                        shift = 0
                    if (shift != 0 && ans.charAt(shift) == ';')
                        shift += 1
                    //console.log('shift'+shift)

                } while (ans.charAt(shift) != 'm' && shift != 0 && shift < ans.length)
                }
                const applyFmt = fon_color || fon_back || bold || under || strike || italic || blink || hide

                if ((!wasEsc || shift > 0) && ans.length > shift || applyFmt) {
                    if (applyFmt)  {
                        ansi_html += '<span style="'
                        if (fon_color ) 
                             ansi_html += 'color:' + fon_color + ';'
                        if (fon_back ) 
                             ansi_html += 'background-color:' + fon_back + ';'
                        if (bold ) 
                             ansi_html += 'font-weight: bold;'
                        if (under ) 
                             ansi_html += 'text-decoration: underline;'
                        if ( strike )
                            ansi_html += 'text-decoration: line-through;'
                       if ( italic )
                            ansi_html += 'font-style: italic;'
                        if ( blink )
                            ansi_html += 'animation:blink 0.75s ease-in infinite alternate!important;'
                        if ( hide )
                            ansi_html += 'opacity: 0.0;'
                        ansi_html += '">' + htmlEncode(ans.substring(shift>0?shift + 1:0)) +'</span>'
                    } else {
                        const lineStr = htmlEncode(ans.substring(shift>0?shift + 1:0))
                        const matches = Array.from(lineStr.matchAll(fileNameReg)); // [...matchAll]
                        if (matches.length > 0) {
                            const file = matches[0].groups.file;
                            const line = matches[0].groups.line;
                            const col = matches[0].groups.col;
                            var path = matches[0].groups.path
                            if (path.startsWith('/') || path.indexOf(':\\') == 1) // current OS root
                                path = path.substring(HOME_LEN + PROJECT_HOME.length+1)
                            path = path.replaceAll('\\', '/')
                            const extraPath = SRC_DIR==''?'':SRC_DIR + '/' //'src/'
                            ansi_html += `<a href="javascript:moveToLineInFile('${path}${extraPath}${file}',${line},${col})">${lineStr}</a>`
                        } else {
                            const urlRegex = /https?:\/\/[^\s]+/g;
                            const htmlText = lineStr.replace(urlRegex, function(url) {
                                  return '<a href="' + url + '" target="_blank">' + url + '</a>';
                            })
                            ansi_html += htmlText
                        }
                    }
                } else {
                    if (ans.charAt(shift) == 'm')
                        shift ++
                    if (ans.length > shift) // TODO refactor
                         ansi_html += htmlEncode(ans.substring(shift))
                }
                wasEsc = true
            }
            //console.log(ansi_html) // debug
            term_frag.innerHTML = ansi_html
        } else {
            let lineStr = htmlEncode(chunk)
            const matches = Array.from(lineStr.matchAll(fileNameReg))
            if (matches.length > 0) {
                const file = matches[0].groups.file;
                const line = matches[0].groups.line;
                const col = matches[0].groups.col;
                var path = matches[0].groups.path
                if (path.startsWith('/') || path.indexOf(':\\') == 1) // current OS root
                    path = path.substring(HOME_LEN + PROJECT_HOME.length+1)
                path = path.replaceAll('\\', '/')
                const extraPath = SRC_DIR==''?'':SRC_DIR + '/' //'src/'
                lineStr = `<a href="javascript:moveToLineInFile('${path}${extraPath}${file}',${line},${col})">${lineStr}</a>`
            }
            term_frag.innerHTML = lineStr
        }
        //cons.appendChild(term_frag)
        appendContent(cons,term_frag)
        if (!noPrompt) {
            // print command prompt
            const prompt = document.createElement("pre")
            prompt.textContent = '\n$'
            appendContent(cons,prompt)
            //cons.appendChild(prompt)
        }
        cons.scrollIntoView({ behavior: "smooth", block: "end" })
     }
     wskt.onclose = (event) => {
        if (notifRecon == 0)
          notifRecon = 500
        if (notifRecon < maxReconn)
          notifRecon *= 2
        if (console && console.log)
            console.log(`Oops, ${event}  reconnecting in ${notifRecon}ms because ${event.reason}`)
        setTimeout(ws_connect, notifRecon)
     }
}

function appendContent(term,el) {
    const lastChild = term.lastElementChild;
    term.insertBefore(el, lastChild);
}

function sendCommand(cmd) {
	   switch (event.key) {
	    case 'Enter':
	         if (wskt && wskt.readyState===WebSocket.OPEN) {
               if (cmd.textContent && cmd.textContent != '\xa0') {
                   if (cmd.textContent.trim() == 'clear' || WIN_SERVER && cmd.textContent.trim() == 'cls') { // 'reset'
                       clearScreen()
                   } else {
                        let inputStr = cmd.textContent.trim()
                        if (inputStr.startsWith('\xa0'))
                            inputStr = inputStr.substring(1)
                        if (inputStr.endsWith('\xa0'))
                            inputStr = inputStr.substring(0, inputStr.length-1)
                         
                        wskt.send(inputStr+'\n')
                   }
                   const commIdx = commandBuffer.indexOf(cmd.textContent)
                   if (commIdx < 0)
                        commandBuffer.push(cmd.textContent)
                   else
                        cmdBufPos = commIdx
			   } else
			  	sendEnter()
			  cmd.textContent = '\xa0'
			  event.preventDefault()
		   } else {
		         console.log('websocket closed')  
		         alert('The server ' + ws_url + ' is currently unreachable')
		   }
	        return
	    case 'ArrowUp':
	        if (commandBuffer.length) {
	           cmdBufPos--
	           if (cmdBufPos < 0)
	              cmdBufPos = commandBuffer.length-1
	        }
	        break
	    case 'ArrowDown':
	        if (commandBuffer.length) {
	           cmdBufPos++
	           if (cmdBufPos > commandBuffer.length-1)
	              cmdBufPos = 0 
	        }
	        break
	    case 'Tab':  // event.keyCode 9 
	        wskt.send(cmd.textContent + '\t')
	        event.preventDefault()
	        return
	    case 'Escape':
            cmd.textContent = '\xa0'
	        event.preventDefault()
	        return
	    case "Backspace":
	    case "Delete":
	        if (cmd.textContent.length == 1) {// prevent complete cleaning the field
	            event.preventDefault()
	            cmd.textContent = '\xa0'
	        }
	        return
	    default:
	       if (event.ctrlKey) {
	          if (event.keyCode == 67) {
	       	   sendCtrlC()
	       	   event.preventDefault()
	          } else if (event.keyCode == 90){
	          	sendCtrlZ()
	          	event.preventDefault()
	          } else if (event.keyCode == 76) {
	         	 clearScreen()
	         	 event.preventDefault()
	          } 
	       }
	       return
	  }
	  if (commandBuffer.length) {
    	 	cmd.innerText = commandBuffer[cmdBufPos]
    	    const range = document.createRange();
            const selection = window.getSelection();
    
            range.selectNodeContents(cmd);
            range.collapse(false); // Collapse to the end
    
            selection.removeAllRanges();
            selection.addRange(range);
    
            cmd.focus();
	  }
	  event.preventDefault()
   }
   
   function sendEnter() {
       if (wskt && wskt.readyState===WebSocket.OPEN) {
		    wskt.send('\n')
		}
   }
   function sendCtrlZ() {
       if (wskt && wskt.readyState===WebSocket.OPEN) {
		   wskt.send('\u001A')
		   document.querySelector('#commandarea').textContent='\xa0'
	   }
   }
   function sendCtrlC() {
       if (wskt && wskt.readyState===WebSocket.OPEN)
		          wskt.send('\x03')
   }
   function clearScreen() {
        const cons = document.querySelector('#terminal')
       //cons.replaceChildren();
        clearColorAttributes()
        while (cons.firstChild.tagName != 'CODE') {
            cons.firstChild.remove()
        }
        const prompt = document.createElement("pre")
        prompt.textContent = '$'
        appendContent(cons,prompt)
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