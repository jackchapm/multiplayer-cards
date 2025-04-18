import * as cdk from 'aws-cdk-lib';
import {RemovalPolicy} from 'aws-cdk-lib';
import * as path from 'path';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import {Construct} from 'constructs';
import {RustFunction} from 'cargo-lambda-cdk';
import {
  DomainName,
  HttpApi,
  HttpMethod,
  HttpStage,
  WebSocketApi,
  WebSocketStage
} from 'aws-cdk-lib/aws-apigatewayv2';
import {HttpLambdaIntegration, WebSocketLambdaIntegration} from 'aws-cdk-lib/aws-apigatewayv2-integrations';
import * as config from '../config.json'
import {AttributeType, BillingMode, Table} from 'aws-cdk-lib/aws-dynamodb';
import {WebSocketLambdaAuthorizer} from 'aws-cdk-lib/aws-apigatewayv2-authorizers';

const BASE_PATH = path.join(__dirname, '..', '..', 'backend')
const CERT_ARN = config.certificateArn

export class MultiplayerCardsStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const table = new Table(this, 'Table', {
      partitionKey: {name: 'pk', type: AttributeType.STRING},
      readCapacity: 5,
      writeCapacity: 5,
      billingMode: BillingMode.PROVISIONED,
      removalPolicy: RemovalPolicy.DESTROY,
    })

    const authDomain = new DomainName(this, 'AuthDomain', {
      domainName: config.devHttpDomain,
      certificate: acm.Certificate.fromCertificateArn(this, 'authCert', CERT_ARN),
    });

    const webSocketDomain = new DomainName(this, 'WebSocketDomain', {
      domainName: config.devWebSocketDomain,
      certificate: acm.Certificate.fromCertificateArn(this, 'webSocketCert', CERT_ARN),
    });

    const authorizerFunction = new RustFunction(this, 'lambda-authorizer', {
      manifestPath: BASE_PATH,
      binaryName: 'lambda-authorizer',
      environment: {
        'JWT_SECRET': config.jwtSecret,
      },
      bundling: {
        cargoLambdaFlags: ['--features', 'build-authorizer'],
      },
    });

    const websocketFunction = new RustFunction(this, 'websocket-function', {
      manifestPath: BASE_PATH,
      binaryName: 'websocket_handler',
      environment: {
        'TABLE_NAME': table.tableName,
        'JWT_SECRET': config.jwtSecret, //todo better secret storage
      },
    });
    table.grantReadWriteData(websocketFunction);

    const authorizer = new WebSocketLambdaAuthorizer('WebSocketAuthorizer', authorizerFunction, {
      identitySource: ['route.request.header.Authorization'],
      authorizerName: "websocket-authorizer",
    });

    // Distinct integrations required per route otherwise execute permissions aren't setup correctly (only for websockets)
    // https://github.com/aws/aws-cdk/issues/22940
    const webSocketApi = new WebSocketApi(this, 'multiplayer-cards-websocket', {
      connectRouteOptions: {integration: new WebSocketLambdaIntegration('WebSocketConnectIntegration', websocketFunction), authorizer: authorizer},
      disconnectRouteOptions: {integration: new WebSocketLambdaIntegration('WebSocketDisconnectIntegration', websocketFunction)},
      defaultRouteOptions: {integration: new WebSocketLambdaIntegration('WebSocketDefaultIntegration', websocketFunction)},
    });

    const webSocketStage = new WebSocketStage(this, 'WebSocketDevStage', {
      webSocketApi: webSocketApi,
      stageName: 'dev',
      domainMapping: {
        domainName: webSocketDomain,
      },
      autoDeploy: true,
    });

    websocketFunction.addEnvironment('WEBSOCKET_ENDPOINT', webSocketStage.callbackUrl);
    webSocketApi.grantManageConnections(websocketFunction);

    const authFunction = new RustFunction(this, 'auth-function', {
      manifestPath: BASE_PATH,
      binaryName: 'auth',
      environment: {
        'TABLE_NAME': table.tableName,
        'JWT_SECRET': config.jwtSecret, //todo better secret storage
      },
    });
    table.grantReadWriteData(authFunction);

    const authApi = new HttpApi(this, 'multiplayer-cards-auth', {
      createDefaultStage: false,
    });

    const authIntegration = new HttpLambdaIntegration('AuthIntegration', authFunction);

    authApi.addRoutes({
      path: '/guest',
      methods: [ HttpMethod.POST ],
      integration: authIntegration,
    });
    authApi.addRoutes({
      path: '/refresh',
      methods: [ HttpMethod.POST ],
      integration: authIntegration,
    });

    new HttpStage(this, 'AuthDevStage', {
      httpApi: authApi,
      domainMapping: {
        domainName: authDomain,
        mappingKey: 'auth',
      },
      stageName: 'dev',
      autoDeploy: true,
    });
  }
}
