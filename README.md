# Rust Development Studio - CGI

## Purpose
Web interfaced IDE for a development primarily in Rust. The IDE can run locally or some cloud machine. 
It can be accessed from any device having an HTML5 capable browser. It's perfect for tablets like Amazon Fire and others.

BTW even Google were inspired by the application and implemented it too as : Code anywhere with Android Studio Cloud. So the RDS already
duplicated by Google.

## Architecture
The client part is quite obvious and utilizes HTML 5, CSS and JavaScript. But the server part resurrects the forgotten CGI technology which is
perfectly suitable for using Rust. The terminal functionality is implemented using the websocket endpoint. So the RDS has
written in Rust, HTML, CSS, and JavaScript and runs on Rust web server - [SimHttp](https://github.com/vernisaz/simhttp/tree/master).

## Implementation
It's a really compact with footprint below 10 MB. All web requests are handled by one Rust CGI executable. The terminal is
a websocket endpoint utilizing the WS-CGI technology.

## Config

The following configuration snippet has to be added to [env.conf](https://github.com/vernisaz/simhttp/blob/master/env.conf) 
of [SimHTTP](https://github.com/vernisaz/simhttp) Rust web server:
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
      {"path":"/terminal",
       "WS-CGI": true,
       "translated": "./../rust_dev_studio/terminal"}]
```
Make sure that *translated* paths are valid in the underline OS and they can be related to the directory the server started from.
File *.home* residing in the same directory, where **rustcgi** executable stored, is required. The file has to specify fully qualified
paths to the _HOME_ directory. (Actually it's any directory where the directory _.rustcgi_ will be landed)

The mapping is used to run the RDS from the dev environment, therefore the access URL
will look like `http://orangepi5:3000/rustcgi/rustcgi/src/html?session=rds&`,
however it will be in a simpler appearance in the standard packaging as
`http://orangepi5:3000/rustcgi/rustcgi`. Port and host name will be dependent
on your environment. Obviously you can change the application web path too.


You are free to use any other web server capable to run CGI scripts. But the terminal can be used only with **SimHTTP**,
because the server only provides the WS CGI support.

There is Java terminal websocket endpoint built on JSR 356 technology. It's out of support but still
can be obtained from [there](https://gitlab.com/tools6772135/rusthub/-/tree/master/src/java/rustcgi).

## Ace Cloud editor
Rust Development Studio is loosely coupled with Ace (Ajax.org Cloud9) Editor.

RDS is coming bundled with some version of the Ace editor. You can always bundle it with any other version of the editor. 
Go to [Ace](https://github.com/ajaxorg/ace-builds/) and download a desired version. A copy of it has to be placed in the directory
**resource/ace**. It's reflected in the install script.

## Packaging
Although you can configure the development studio yourself accordingly to a web server and other components,
there is the predefined packaging.

It includes all required **RDS** components. Just launch `./rds.sh` or `.\rds.bat` on _Windows_
and you can start using the **RDS**. The access URL's stored in `rds.url`.

## Building RDS components

The [RustBee](https://github.com/vernisaz/rust_bee) scripting tool is used for building. Obtain it first.

The following crates will be required to be built first:

- The [Common building scripts](https://github.com/vernisaz/simscript) (it isn't a crate, but building scripts)
- The [SimWeb](https://github.com/vernisaz/simweb)
- The [Simple Time](https://github.com/vernisaz/simtime)
- The [SimRan](https://github.com/vernisaz/simran) 
- The [Simple Thread Pool](https://github.com/vernisaz/simtpool)

Common scripts are containing scripts only and shouldn't be built. 
The **crates** directory on the level of repository directories has to be created prior building unless it's
already exists. Run **rb** in each component repository. 

[bee.7b](./bee.7b) script used for building **RDS**, and [bee-term.7b](./bee-term.7b) script used to build the terminal.

## Working tips

You will see an empty page when first time pointed a browser to **RDS** URL. Select menu *File/Project/New...*.
Navigate to a project directory and then *Apply* for the new project.
Open just created project from menu *File/Project/\<name\>* then. You can start to
navigate over the project tree, open and edit files, build its components and so on.

If you do not have the project checked out yet, then you can check it out first, and then to set the project root 
in the settings. You can execute underline OS commands as *mkdir*, *git clone*, and others in the terminal window. 

When you use **Cargo** to build a Rust project, make sure to set `CARGO_TERM_COLOR` env variable to `always`. It
is controlled also by `term.color` of Cargo settings. For example:
```
[term]
color=always
```
## Cloud and multi users install
If you plan to use the IDE on Cloud in a multi users environment, then you need to provide a proxy server in the front which provides:
- SSL to plain connection conversion
- A user authentication
- URL translation

For example, a Cloud URL in the form - `https://ide.cloud.com/user-name` gets translated to `http://innerhost:3000/rustcgi/rustcgi`,
the IDE uses only URLs in a relative form and doesn't need to know how an actual URL looks when it got accessed
`

## Browser compatibly

**RDS** uses HTML 5 features presented in popular browsers today. 
Compatibility was tested using the latest **Firefox**, **Edge**, **Safari**, and **Amazon Silk** browsers.
More likely, other browsers will work too. If you encounter problems with your browser,
then, please, report them to the author.

## Like the IDE but Rust
You can add any other language in support. Open [main.html](./src/html/main.html) and navigate to
line 267 to see
```javascript
const EDITOR_MODE = {
```
Add more modes accordingly file extension of the code type of your interest.

## Known problems

1. *datalist* seems isn't supported by the Silk browser, so an auto suggest won't work on Fire tablets
2. files can stop to be opened from the left navigation panel (work around - select
_Refresh Proj_ from _Edit_ menu or reload the project page)

## Reading about

1. Some light on using [technologies](https://www.linkedin.com/pulse/new-life-old-technologies-dmitriy-rogatkin-nznpc/).


## Help wanted

If you like the IDE and want to contribute to their development, please contact me using the standard
github communication way. Entry level people are welcome.

Current tasks:

1. implement a communication with ChatGPT and similar systems