const USE_GAGS = true;

window.addEventListener("DOMContentLoaded", () => {


  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;
  const { message, open } = window.__TAURI__.dialog;
  const { attachConsole } = window.__TAURI__.log;

  const detach = attachConsole();


  let svgEl;
  let statusEl;
  let logsEl;
  let clearEl;
  let cmdBuildBtnElt;
  let cmdBuildTaskElt;

  /// For each rule, store the div that contains its logs
  let rulesDiv = new Map();
  /// Same for each task
  let tasksDivs = new Map();

  let ruleProgressEl;
  let ruleProgressLabelEl;

  let rules_list = [];

  function update_rules_list() {
    const max_len = 10;

    let content = rules_list.join(", ");
    let len = content.length;

    if (len > max_len) {
      content = "..." + content.substring(len - max_len + 3);
    }

    ruleProgressLabelEl.innerHTML = content;

  }


  function add_log(elt, txt, kind) {
    console.log("txt", txt);
    let content = txt.replaceAll("\n", "<br/>");
    //let content = txt;
    elt.innerHTML += "<span class=\"" + kind + "\">" + content + "</span>";
  }


  function clearLogs() {
    logsEl.innerHTML = "";
  }

  function executeManualTask() {
    clearLogs();
    invoke("execute_manual_task", {'task': cmdBuildTaskElt.value})
      .then((evt) => console.log(evt))
      .catch((evt) => {
        console.log(evt);
        add_log(logsEl, evt, "stderr")
      });
  }


  svgEl = document.querySelector("#svgContainer");
  statusEl = document.querySelector("#statusContainer");

  ruleProgressEl = document.querySelector("#ruleProgress");
  ruleProgressLabelEl = document.querySelector("#ruleProgressLabel");

  logsEl = document.querySelector("#logs");
  clearEl = document.querySelector("#clear_button");

  cmdBuildBtnElt = document.querySelector("#cmdBuildBtn");
  cmdBuildTaskElt = document.querySelector("#cmdBuildTask");

  // no context menu
  window.document.addEventListener("contextmenu", (event) => {
    event.preventDefault();
  });


  if (USE_GAGS) {
    window.setInterval(() => {
      invoke("empty_gags")
    }, 1000 / 100);
  }

  cmdBuildBtnElt.addEventListener("click", (event) => {
    executeManualTask();
  });

  cmdBuildTaskElt.addEventListener("keyup", (event) => {
    if (event.key === 'Enter' || event.keyCode === 13) {
      executeManualTask();
    }
  });

  // TODO consider all these events can happen in parallel as soon as
  //      we'll parallelized tasks execution
  // The system starts to execute a rule (that contains potentially several tasks)
  listen('event-start_rule', (event) => {
    const rule = event.payload.rule;
    ruleProgressEl.max = event.payload.out_of;
    rules_list.push(rule);
    update_rules_list();

    let ruleDiv = window.document.createElement("div");
    ruleDiv.innerHTML = "<p> Rule: " + event.payload.rule + "</p>";
    logsEl.appendChild(ruleDiv);
    rulesDiv.set(rule, ruleDiv);
  });

  // The system has stopped to execute a rule
  listen("event-stop_rule", (event) => {
    const rule = event.payload;
    const idx = rules_list.indexOf(rule);
    rules_list.splice(idx, 1);
    ruleProgressEl.value += 1;

    update_rules_list();
    if (rules_list.length == 0) {
      ruleProgressEl.max = 0;
      ruleProgressEl.value = 0;
    }

    rulesDiv.delete(rule);
  });

  // A rule failed
  listen("event-failed_rule", (event) => {
    let rule = event.payload;
    rulesDiv.get(rule).firstChild.innerHtml += "[failure]";
    //rulesDiv.delete(rule);
  });

  // A task is starting for a given rule
  listen("event-task_start", (event) => {
    console.log(event);
    const opt_rule = event.payload.rule;
    const cmd = event.payload.cmd;
    const task_id = event.payload.task_id;

    let taskDiv = window.document.createElement("div");
    taskDiv.innerHTML += "<p><code>" + cmd + "</code></p>";

    let logDiv = window.document.createElement("div");
    taskDiv.appendChild(logDiv);

    console.log("Add", [taskDiv, logDiv], "to", task_id);
    tasksDivs.set(task_id, [taskDiv, logDiv]);
    console.log("Gives", tasksDivs);

    /// TODO handle the case where there is no rule
    console.log(rulesDiv.get(opt_rule));
    rulesDiv.get(opt_rule).appendChild(taskDiv);
  });

  // A task has stopped
  listen("event-task_stop", (event) => {
    console.log(event);
    const opt_rule = event.payload.rule;
    const task_id = event.payload.task_id;
    const duration = event.payload.duration_milliseconds;

    const human_duration = (duration / 1000) + " s";

    let [taskDiv, logDiv] = tasksDivs.get(task_id);
    //  tasksDivs.delete(task_id);

    taskDiv.firstChild.innerHtml += " " + human_duration;
  });

  // Standard stdout for a given task
  listen("event-task_stdout", (event) => {
    console.log(event);
    console.log(tasksDivs);

    const rule = event.payload.rule;
    const task_id = event.payload.task_id;
    const content = event.payload.content;

    console.assert(tasksDivs.has(task_id));
    let [taskDiv, logDiv] = tasksDivs.get(task_id);
    add_log(logDiv, content, "stdout");
  });

  // Standard stderr for a given task
  listen("event-task_stderr", (event) => {
    console.log(event);
    const rule = event.payload.rule;
    const task_id = event.payload.task_id;
    const content = event.payload.content;

    let [taskDiv, logDiv] = tasksDivs.get(task_id);
    add_log(logDiv, content, "stderr");

  });

  // Global stdout
  listen("event-stdout", (event) => {
    console.log(event);
    add_log(logsEl, event.payload, "stdout");
  });

  // global stderr
  listen("event-stderr", (event) => {
    console.log(event);
    add_log(logsEl, event.payload, "stderr");
  });




  listen("request-execute_target", (event) => {
    execute_target(event.payload);
  });
  listen("request-load_build_file", async (event) => {
    console.log("request load", event);
    load_build_file(event.payload);
  });
  listen("request-reload", (event) => {
    console.log("request-reload", event);
    invoke("reload_file")
      .then((msg) => { })
      .catch((msg) => { });
  });
  listen("request-select_cwd", async (event) => {
    console.log("request-select_cwd", event);
    const dname = await open({
      "title": "Select the working directory",
      "directory": true
    });

    console.log("Try to select dir", dname)
    invoke("select_cwd", {"dname": dname})
      .then((msg) => { 

        svgEl.innerHTML = ""; // if we select a specific directory we cannot use a build script
        statusEl.innerHTML = "Work directory: " + dname;
      })
      .catch((msg) => {add_log(logsEl, msg, "stderr");})
  });
  listen("request-open", async (event) => {
    const fname = await open({
      "title": "Open a BNDbuild script",
      "filters": [
        {
          "name": "BNDBuild files",
          "extensions": ["build", "bnd", "yml"]
        }
      ]
    });

    console.log("try to open", fname);

    load_build_file(fname);

  });

  listen("request-clear", (event) => {
    invoke("clear_app", { soft: event.payload })
      .then((msg) => console.log(msg))
      .catch((msg) => console.error(msg));
  });



  async function load_build_file(fname) {
    clearLogs();
    statusEl.innerHTML = "<span>Loading " + fname + "</span>";
    window.document.body.style.cursor = "wait";
    svgEl.innerHTML = "";
    invoke("load_build_file", { fname: fname })
      .then((msg) => window.document.body.style.cursor = "default")
      .catch((msg) => {
        console.error(msg);
        window.document.body.style.cursor = "default"
  });
  }



  function execute_target(tgt) {
    // ensure log area is cleared
    clearLogs();

    // Execute the target and handle success and error
    invoke("execute_target", { tgt: tgt })
      .then(async (msg) => {
        console.log("execute_target success", msg);
        await message("Build success", { title: 'BNDBuild', kind: 'info' });
      })
      .catch(async (msg) => {
        console.log("execute_target failed", msg);
        add_log(logsEl, msg, "stderr"); // TODO make the rust code generate events ! we do not have ot handle it now.
        await message(msg, { title: 'BNDBuild', kind: 'error' });
      });
  }



  // When a bndbuild file is loaded, it is necessary
  // to replace the svg representation and  add the 
  // handlers to manage the clicks on targets
  listen('file-loaded', (event) => {
    console.log(event);
    // Update the interface according to the loaded file
    svgEl.innerHTML = event.payload.svg;
    statusEl.innerHTML = "Build file:" + event.payload.fname;

    // Setup the events listeners to launch the build
    // All targets are stored within an anchor
    const links = svgEl.querySelectorAll("a");
    links.forEach(link => {
      // retreive the target to build
      const tgt = link.getAttribute("xlink:title");

      // disable the normal behavior of the link
      link.removeAttribute("xlink:href");

      // we add the click listener to the <g> tag that contains the anchor
      const parent = link.parentElement;
      parent.style.pointerEvents = "bounding-box"; // ensure whole g is clicable
      parent.style.cursor = "pointer";

      // handle left click
      parent.addEventListener("click", (e) => { // handle the click
        execute_target(tgt);
      });

      // handle right click
      parent.addEventListener("contextmenu", (e) => {
        invoke("open_contextual_menu_for_target", { tgt: tgt });
      })

    });


    // we can assume that rust code from the menu is finished
    // and we can update the menu (especially the list of opened files)
    invoke("update_menu")
      .then((msg) => console.log("menu update success", msg))
      .catch((msg) => console.log("menu update failed", msg));


    clearLogs();

    clearEl.addEventListener("click", () => {
      // TODO when done during a construction, it is needed to do it differently
      // by inidividually clearing the dom element but not removing them
      clearLogs();
    })
  });


});
