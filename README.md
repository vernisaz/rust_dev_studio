# Rust Development Studio - CGI

## Purpose
Web interfaced IDE for a development primarily in Rust. The IDE can run locally or on some cloud machine. 
It can be accessed from any device having an HTML5 capable browser. It's perfect for tablets like Amazon Fire and others.

The approach of web based IDE becomes popular nowadays, for example - Code anywhere with Android Studio Cloud.

## Architecture
The client part is quite obvious and utilizes HTML 5, CSS and JavaScript. But the server part resurrects the forgotten CGI technology which is
perfectly suitable for using Rust. The terminal functionality is implemented using a websocket endpoint. So the RDS has
written in Rust, HTML, CSS, and JavaScript and runs on Rust web server - [SimHttp](https://github.com/vernisaz/simhttp/tree/master).

## Implementation
It's a really compact with a footprint below 10 MB. All web requests are handled by one Rust CGI executable. The terminal is
a websocket endpoint utilizing the WS-CGI technology.

## Config

The following configuration snippet has to be added to [env.conf](https://github.com/vernisaz/simhttp/blob/master/env.conf) 
of the [SimHTTP](https://github.com/vernisaz/simhttp) Rust web server:
```json
"mapping" : [
  {"path":"/cgires/resource",
       "CGI": false,
       "translated": "./../rust_dev_studio/src/html"},
       {"path":"/cgires/resource/ace",
       "CGI": false,
       "translated": "./../side/ace-builds/src-min-noconflict"},
      {"path":"/rustcgi",
       "CGI": true,
       "translated": "./../rust_dev_studio"},
      {"path":"/cgires/resource/terminal",
       "CGI": false,
       "translated": "./../simterminal/src/js"},
      {"path":"/terminal",
       "WS-CGI": true,
       "translated": "./../rust_dev_studio/terminal"}]
```
The mapping is used to run the RDS from the dev environment, therefore an access URL
looks like `http://orangepi5:3000/rustcgi/rustcgi/src/html?session=rds&`,
however a simpler form is used in the standard packaging as
`http://orangepi5:3000/rustcgi/rustcgi`. A port and a host name will depend
on your environment. Even CGI mapping can be changed.

Make sure that *translated* paths are valid in the underline OS and they can be related to the directory the server started from.

File *.config* residing in the same directory, where **rustcgi** executable is stored, is required.
The file has to specify a fully qualified
path to the common config directory where RDS will land the directory _.rds_.

Directory _.rds_ may contain file _.workspace_ which contains a path to the directory where
all projects will be landed. If there is no such file presented, then the workplace directory 
will be the same as specified in the _.config_ file. For example a user _HOME_ directory
can be provided in the _.config_ file.


You are free to use any other web server capable to run CGI scripts. But the terminal can be used only with the **SimHTTP**,
because the server provides the WS CGI support.

There is Java terminal websocket endpoint built on JSR 356 technology. It's out of support, but still
can be obtained from [there](https://gitlab.com/tools6772135/rusthub/-/tree/master/src/java/rustcgi).

## Ace Cloud editor
The Rust Development Studio is loosely coupled with Ace (Ajax.org Cloud9) Editor.

RDS is coming bundled with some version of the Ace editor. You can always bundle it with any other version of the editor. 
Go to [Ace](https://github.com/ajaxorg/ace-builds/) and download the desired version. A copy of it has to be placed in the directory
**resource/ace**. It's reflected in the installation script.

## Building RDS components

The [RustBee](https://github.com/vernisaz/rust_bee) scripting tool is used for building. Obtain it first.

The following crates are required for building the product:

- The [Common building scripts](https://github.com/vernisaz/simscript) (it isn't a crate, but building scripts)
- The [SimWeb](https://github.com/vernisaz/simweb)
- The [Simple Time](https://github.com/vernisaz/simtime)
- The [SimRan](https://github.com/vernisaz/simran) 
- The [Simple Thread Pool](https://github.com/vernisaz/simtpool)
- The [Simple Config](https://github.com/vernisaz/simconfig)

Common scripts are containing scripts used for building crates. 
The **crates** directory on the level of repository directories has to be created prior building unless it's
already exists. Run **rb** in the each crate repository. 

Finally
[bee.7b](./bee.7b) script used for building **RDS**, and [bee-term.7b](./bee-term.7b) script used to build the **terminal**.

The terminal functionality is separated in the crate [SimTerminal](https://github.com/vernisaz/simterminal), therefore build it before
building the terminal and then build the terminal. Note that the terminal has one extra dependency [simple Color](https://github.com/vernisaz/simcolor).

## Packaging
Although you can configure the development studio yourself accordingly to a web server and other components location,
there is the predefined packaging.

It includes all required **RDS** components. Just launch `./rds.sh` or `.\rds.bat` on _Windows_
and you can start using the **RDS**. The access URL's stored in `rds.url`.

There is a RustBee [script](https://github.com/vernisaz/rust_dev_studio/blob/master/install/bee.7b) to
create the standard packaging.

## Working tips

You will see an empty page when first time pointed a browser to the **RDS** URL. Select menu *File/Project/New...*.
Navigate to a project directory and then *Apply* for the new project.
Open the just created project from menu *File/Project/\<name\>* then. You can start to
navigate over the project tree, open and edit files, build its components and so on.

If you do not have the project checked out yet, then you can check it out first, and then to set the project root 
in the settings. You can execute underline OS commands as *mkdir*, *git clone*, and others in the terminal panel. 

When you use **Cargo** to build a Rust project, make sure to set `CARGO_TERM_COLOR` env variable to `always`. It
is controlled also by `term.color` of Cargo settings. For example:
```
[term]
color=always
```
## Cloud and multi users install
If you plan to use the IDE on a Cloud in the multi users environment, then you need to provide a proxy server in the front of which provides:
- Decryption/Re-encryption (SSL Forward Proxy)
- A user authentication
- URL translation
- Reject a brutal force and other cyber attacks

For example, a Cloud URL in the form - `https://ide.cloud.com/user-name` gets translated to `http://internalhost:3000/`,
the IDE uses only URLs in a relative form and doesn't need to know how an actual URL looks when it got accessed.

## Browser compatibly

**RDS** uses HTML 5 features presented in popular browsers today. 
Compatibility was tested using the latest **Firefox**, **Edge**, **Safari**, and **Amazon Silk** browsers.
More likely, other browsers will work too. If you encounter problems with your browser,
then, please, report them to the author.

## Like the IDE but Rust
You can add any other language in the support. Open [main.html](https://github.com/vernisaz/rust_dev_studio/blob/229f4862dc61c7aeb480769df776109763f3d945/src/html/main.html#L264) and navigate to
around line 278 to see
```javascript
const EDITOR_MODE = {
```
Add more modes accordingly file extension of the code type of your interest.

If you need to navigate in the source code from error messages in the terminal, look for 
```javascript
var fileNameReg
```
around line 379. Add a desired language file extension in the regular expression definition.

## Version
The current version is [1.50.04](https://github.com/vernisaz/rust_dev_studio/releases/tag/1.45.01). You can check out the current development code,
however it may contain some bugs.

## Known problems

1. files can stop to be opened from the left navigation panel (work around - select
_Refresh Proj_ from _Edit_ menu or reload the project page)

## Reading about

1. Some light on using [technologies](https://www.linkedin.com/pulse/new-life-old-technologies-dmitriy-rogatkin-nznpc/).


## Help wanted

If you like the IDE and want to contribute to their development, please contact me using the standard
github communication way. Entry level people are welcome.

Current tasks:

1. Build AI engine on different principles than ChatGPT