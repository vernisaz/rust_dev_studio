# Rust Development Studio - CGI

## Purpose
Web interfaced IDE for app development in different tech stacks, but primary Rust targeted. The IDE can run locally or some cloud machine. 
It can be accessed from any device having a HTML5 capable browser. It's perfect for tablets as Amazon fire.

## Architecture
The client part is quite obvious and utilizes HTML 5, CSS and JavaScript. But the server part resurrects the forgotten CGI technology which is
perfectly suitable for using Rust. The terminal functionality is implemented using the websocket. So the RDS has
written in Rust, HTML, CSS, and JavaScript.

## Implementation
It's a really compact with footprint below 10 MB. All web requests are handled by one Rust CGI executable. The terminal is
a websocket endpoint represented by the WS-CGI technology and implemented as a Rust application too.

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
Make sure that *translated* paths are valid in the underline OS.
File *.home* residing in the same directory where **rustcgi** is required. The file has to specify fully qualified
paths to the _HOME_ directory.

The mapping is used to run the RDS from the dev environment, therefore URL
will look like `http://orangepi5:3000/rustcgi/rustcgi/src/html?session=rds&`,
however it will be in a simpler form in the standard packaging as
`http://orangepi5:3000/rustcgi/rustcgi`. Port and host name will be dependent
on your environment. Obvioustly you can change the root web path also.


You are free to use any other web server capable to run CGI scripts. Rust terminal can be used only with **SimHTTP**,
because only this server provides the WS CGI support.

There is Java terminal websocket endpoint built on JSR 356 technology. It's out of support but still
can be obtained [from](https://gitlab.com/tools6772135/rusthub/-/tree/master/src/java/rustcgi).

## Ace Cloud editor
Rust Development Studio is loosely coupled with Ace (Ajax.org Cloud9) Editor.

RDS is coming bundled with some version of the Ace editor. You can always bundle it with any other version of the editor. 
Go to [Ace](https://github.com/ajaxorg/ace-builds/) and download a desired version. A copy of it has to be placed in the directory
**resource/ace**. It's reflected in the install script.

## Packaging
Although you can configure the development studio yourself accordingly to a web server and other components,
there is predefined packaging.

The standard packaging includes all required **RDS** components. Just launch `./rds.sh` or `.\rds.bat` on _Windows_
and you can start using the **RDS**. The access URL's stored in `rds.url`.

## Building rustcgi components

[RustBee](https://github.com/vernisaz/rust_bee) scripting tool is used for building. Obtain it first.

The following crates will be required to be built first:

- The [common building scripts](https://github.com/vernisaz/simscript)
- The [SimWeb](https://github.com/vernisaz/simweb)
- The [simple time](https://github.com/vernisaz/simtime)
- The [SimRan](https://github.com/vernisaz/simran) 
- The [Simple Thread Pool](https://github.com/vernisaz/simtpool)
- 

Common scripts used for building other components (crates). Run **rb** in each component root directory. **crates** directory
needs to be created first where built components will be stored.

[bee.7b](./bee.7b) script used for building **RDS**, and [bee-term.7b](./bee-term.7b) script used to build the terminal.

## Working tips

You will see an empty page when first time pointed a browser to **RDS** URL. Select menu *Settings* or *File/Project/New...*.
Navigate to a project directory and then *Save* the settings or *Apply* for the new project.
Open just created project from *File/Project* then. You can start to
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
If you plan to use the IDE on a Cloud in a multi users environment, then you need to provide a proxy server in a front which provides:
- SSL to unsecured connection
- A user authentication
- URL translation

For example, a Cloud URL in the form - `https://ide.cloud.com/user-name` gets translated to `http://innerhost:3000/rustcgi/rustcgi`,
the IDE uses only URLs in a relative form and doesn't need to know how an actual URL looks when it got accessed
`

## Browser compatibly

**RDS** uses HTML 5 features and compatible with browsers supporting them. 
Compatibility was tested using the latest **Firefox**, **Edge**, **Safari**, and **Amazon Silk** browsers.
More likely, other browsers will work too. If you encounter problems with any browser,
then please report them to the author.

## Known problems

1. *datalist* seems isn't supported by the Silk browser, so an auto suggest won't work on Fire tablets
2. files stopped be opened from the left navigation panel (work around - select
_Refresh Proj_ from _Edit_ menu or reload the project)

## Reading about

1. Some light on using [technologies](https://www.linkedin.com/pulse/new-life-old-technologies-dmitriy-rogatkin-nznpc/).


## Help wanted

If you like the IDE and want to contribute to their development, please contact me using the standard
github communication way. Entry level people are welcome.

Current tasks:

1. implement a communication with ChatGPT and similar systems