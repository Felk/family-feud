<script lang="ts">
    import type {Answer} from "./stores";
    import {isGamemaster} from "./stores";

    export let answer: Answer;

</script>

<style>
    div.answer-container {
        display: inline-flex;
        width: 15em;
        height: 2em;

        margin: 0.5em;

        font-size: 1.5em;
        align-content: center;
        align-items: center;
        justify-content: center;
        text-align: center;

        background-color: #0065ad;
        outline: 2px solid #6299c0;
        border: 2px solid #003b6e;
    }

    .answer-id {
        display: flex;
        align-items: center;
        justify-content: center;

        height: 2em;
        width: 2em;

        border-radius: 50%;
        background-color: #003255;
    }

    .answer {
        display: grid;
        width: 100%;
        height: 100%;
        grid-template-areas: "text votes";
        grid-template-columns: auto 2em;
    }

    .answer-text {
        grid-area: text;
        text-transform: uppercase;

        display: flex;
        justify-content: center;
        align-items: center;
    }

    .answer-text.hidden {
        color: #ffffff66;
        text-decoration: line-through;
    }

    .answer-votes {
        grid-area: votes;
        background-color: rgba(0, 0, 0, 0.5);
        font-size: 1.5em;
    }
</style>

<div class="answer-container">
    {#if answer.shown}
        <div class="answer">
            <span class="answer-text">{answer.text}</span>
            <span class="answer-votes">{answer.votes}</span>
        </div>
    {:else if $isGamemaster}
        <div class="answer">
            <span class="answer-text hidden">{answer.text}</span>
            <span class="answer-votes">{answer.votes}</span>
        </div>
    {:else}
        <div class="answer-id">{answer.id}</div>
    {/if}
</div>
