#!/usr/bin/env node
import * as cdk from 'aws-cdk-lib';
import * as config from '../config.json'
import { MultiplayerCardsStack } from '../lib/multiplayer-cards-stack';

const app = new cdk.App();
const stack = new MultiplayerCardsStack(app, 'MultiplayerCardsStack', {
  env: { account: process.env.CDK_DEFAULT_ACCOUNT, region: process.env.CDK_DEFAULT_REGION },
});
cdk.Tags.of(stack).add('awsApplication', config.applicationTag);