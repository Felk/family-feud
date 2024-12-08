import type {Writable} from "svelte/store";
import {writable} from "svelte/store";

export interface Payload {
    type: string,
    data: any,
}

export interface UpdateProperty {
    name: string,
    data: any,
}

const createRemoteStore = <T>(key: string, defaultValue: T = null): Writable<T> => {
    const store = writable(defaultValue);

    document.addEventListener("remotestore_recv", (evt: CustomEvent<Payload>) => {
        const {type, data} = evt.detail;
        if (type == "update_property") {
            let updateProperty = data as UpdateProperty;
            if (updateProperty.name == key) {
                store.set(updateProperty.data as T);
            }
        }
    });

    const newSet = (value: T) => {
        const payload: Payload = {
            type: "update_property",
            data: {
                name: key,
                data: value
            }
        };
        document.dispatchEvent(new CustomEvent<Payload>("remotestore_send", {detail: payload}))
    };

    return {
        subscribe: store.subscribe,
        set: newSet,
        update: null,
    };
}

export default createRemoteStore;
