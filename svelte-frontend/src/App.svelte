<script lang="ts">
    import type {Payload} from "./remotestore";
    import {type Answer, answers, connectionStatus, ConnectionStatus, isGamemaster, title} from "./stores";
    import AnswerComponent from "./AnswerComponent.svelte";

    const connect = () => {
        console.debug("connecting to websocket...")
        $connectionStatus = ConnectionStatus.Connecting;
        const ws = new WebSocket('ws://localhost:8030');
        const listener = (evt: CustomEvent<Payload>) => {
            ws.send(JSON.stringify(evt.detail));
        };
        ws.onopen = function () {
            $connectionStatus = ConnectionStatus.Connected;
            console.debug("websocket connection established")
            document.addEventListener("remotestore_send", listener);
        };

        ws.onmessage = function (e) {
            document.dispatchEvent(new CustomEvent<Payload>("remotestore_recv", {detail: JSON.parse(e.data)}))
        };

        ws.onclose = function (e) {
            $connectionStatus = ConnectionStatus.Disconnected;
            document.removeEventListener("remotestore_send", listener);
            console.log('Socket is closed. Reconnect will be attempted in 1 second.', e.reason);
            setTimeout(function () {
                connect();
            }, 1000);
        };

        ws.onerror = function (e) {
            $connectionStatus = ConnectionStatus.Disconnected;
            console.error('Socket encountered error: ', e, 'Closing socket');
            ws.close();
        };
    }

    connect();

    const drop_handler = (ev) => {
        ev.preventDefault();
        // const files = await getAllFileEntries(ev.dataTransfer.items);

        const file = ev.dataTransfer.files[0];
        const reader = new FileReader();
        reader.onload = e => {
            const text: string = e.target.result as string;
            const lines = text
                .split("\n")
                .map(value => value.trim())
                .filter(value => value != "");
            const newTitle = lines[0];
            let newAnswers = [];
            let id = 1;
            for (const l of lines.slice(1)) {
                const match = /(.*?)\s*\((\d+)\)\s*/g.exec(l);
                const answer: Answer = {
                    id: id++,
                    shown: false,
                    text: match[1],
                    votes: parseInt(match[2]),
                };
                newAnswers.push(answer);
            }
            console.log(newTitle);
            console.log(newAnswers);
            $title = newTitle;
            $answers = newAnswers;
        };
        reader.readAsText(file);

    }
</script>

<style>
    .toggle-shown {
        font-size: 2em;
        /*background: none;*/
        /*border: none;*/
        /*color: white;*/
    }
</style>

<main style="min-height: 100%; min-width: 100%"
      on:drop={drop_handler}
      on:dragover={e => e.preventDefault()}>
    <input id="gamemaster-checkbox" type="checkbox" bind:checked={$isGamemaster}/>
    <label for="gamemaster-checkbox">Gamemaster mode</label>

    {#if $connectionStatus === ConnectionStatus.Connected}
        <div>✅ Connected to server</div>
    {:else if $connectionStatus === ConnectionStatus.Connecting}
        <div>⚠️ Connecting to server...</div>
    {:else if $connectionStatus === ConnectionStatus.Disconnected}
        <div>⚠️ Not connected to server</div>
    {/if}

    <h1>{$title}</h1>
    <div>
        {#each $answers as answer (answer.id)}
            <AnswerComponent answer={answer}/>
            {#if $isGamemaster}
                {#if answer.shown}
                    <button class="toggle-shown" on:click={() => answer.shown = false}>👁</button>
                {:else}
                    <button class="toggle-shown" on:click={() => answer.shown = true} style="opacity: 40%">👁</button>
                {/if}
            {/if}
        {/each}
    </div>
</main>
