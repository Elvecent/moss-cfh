import { writable } from "svelte/store";
import { Actor, HttpAgent } from "@dfinity/agent";
import { AuthClient } from "@dfinity/auth-client";
import { idlFactory } from "../declarations/backend/backend.did.js";
import type { ActorSubclass, Identity } from "@dfinity/agent";
import type { _SERVICE } from "../declarations/backend/backend.did.ts";

type OptionsType = {
  agentOptions?: import("@dfinity/agent").HttpAgentOptions;
  actorOptions?: import("@dfinity/agent").ActorConfig;
};

type ReturnType = {
  actor: import("@dfinity/agent").ActorSubclass<import("../declarations/backend/backend.did.ts")._SERVICE>,
  loginII: () => Promise<boolean>,
  logoutII: () => Promise<boolean>
};

async function getIdentity(): Promise<Identity>{
  try {
    const authClient = await AuthClient.create();
    const identity = authClient.getIdentity();
    if (!identity) {
      throw new Error('Identity not found');
    }
    return identity;
  } catch (error) {
    throw new Error('Failed to obtain Identity');
  }
}

let identity = getIdentity();

export function createActor(options?:OptionsType): ReturnType {

  const hostOptions = {
    host:
      process.env.DFX_NETWORK === "ic"
        ? `https://${process.env.CANISTER_ID_BACKEND}.ic0.app`
        : `http://${process.env.CANISTER_ID_BACKEND}.localhost:4943`,
  };
  
  if (!options) {
    options = { agentOptions: hostOptions };
  } else if (!options.agentOptions) {
    options.agentOptions = hostOptions;
  } else if (options.agentOptions) {
    options.agentOptions.host = hostOptions.host;
  }

  if(options.agentOptions !== undefined) { 
    options.agentOptions.identity = identity;
  }
  
  const agent = new HttpAgent({ ...options.agentOptions });
  
  if (process.env.DFX_NETWORK !== "ic") {
    agent.fetchRootKey().catch((err) => {
      console.warn(
        "Unable to fetch root key. Check to ensure that your local replica is running"
      );
      console.error(err);
    });
  }

  return Actor.createActor(idlFactory, {
    agent,
    canisterId: process.env.CANISTER_ID_BACKEND,
    ...options?.actorOptions,
  });
}

export const ic = writable<ReturnType>({
  actor: createActor() as unknown as ActorSubclass<_SERVICE>,
  loginII: loginII,
  logoutII: logoutII
});

export async function loginII(): Promise<boolean> {
  const authClient = await AuthClient.create();
  let PUBLIC_INTERNET_IDENTITY_CANISTER_ID : string;
  let iiUrl : string;

  if (process.env.DFX_NETWORK !== "ic"){
    PUBLIC_INTERNET_IDENTITY_CANISTER_ID = process.env.CANISTER_ID_INTERNET_IDENTITY;
    iiUrl = `http://${PUBLIC_INTERNET_IDENTITY_CANISTER_ID}.localhost:4943`;
  } else {
    iiUrl = 'https://identity.internetcomputer.org/#authorize';
  }

  let userIsAuthenticated = false;
  await new Promise<void>((resolve, reject) => {
    authClient.login({
      identityProvider: iiUrl,
      onSuccess: resolve,
      onError: reject,
    }).then(() => {
      userIsAuthenticated = true;
    }).catch((e) => {
      console.log("login error", e)
    });
  });

  if(userIsAuthenticated){
    identity = new Promise<Identity>((resolve, reject) => {
      const identity = authClient.getIdentity();
      if (identity) { resolve(identity); } else { reject("No identity found"); }
    });

    let options = {agentOptions: {identity: identity}}; 
    ic.update((state) => {
      const newActor = createActor(options) as any; 
      return {...state, actor: newActor};
    });
  }
  
  return userIsAuthenticated;
};

export async function logoutII(): Promise <boolean> {
  const authClient = await AuthClient.create();
  authClient.logout();

  let userIsAuthenticated = true;
  if(!authClient.isAuthenticated()) {
    userIsAuthenticated = false;
  }

  return userIsAuthenticated;
}