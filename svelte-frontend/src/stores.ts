import createRemoteStore from "./remotestore";
import type {Writable} from "svelte/store";
import {writable} from "svelte/store";

export interface Answer {
    id: number,
    text: String,
    votes: number,
    shown: boolean,
}

export const title: Writable<string> = createRemoteStore("title");
export const answers: Writable<Answer[]> = createRemoteStore("answers", []);

export const isGamemaster: Writable<boolean> = writable(false);

export enum ConnectionStatus {Disconnected, Connecting, Connected}
export const connectionStatus: Writable<ConnectionStatus> = writable(ConnectionStatus.Disconnected);
